use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

pub const MODEL_SHA256: &str = "832e7bb2302c3cd67c818ca4fe9dcbedf696d3070dab5463127263ec4db9899f";
pub const MODEL_ID: &str = "Lykon/dreamshaper-xl-lightning";
pub const RUNNER_COMMIT: &str = "5ef4a75";
pub const MAX_PROMPT_BYTES: usize = 16 * 1024;
pub const MAX_HISTORY: usize = 8;

#[derive(Clone, Debug, Deserialize)]
pub struct AiPackManifest {
    pub format: u32,
    pub runner_commit: String,
    pub model: PackFile,
    pub lora: Option<PackFile>,
    #[serde(default)]
    pub runtime: Vec<PackFile>,
    pub workers: Workers,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PackFile {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Workers {
    pub cuda: Option<PackFile>,
    pub vulkan: Option<PackFile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationQuality {
    Preview,
    Final,
}

impl GenerationQuality {
    pub fn timeout(self) -> Duration {
        match self {
            Self::Preview => Duration::from_secs(120),
            Self::Final => Duration::from_secs(240),
        }
    }

    pub fn steps(self) -> u32 {
        match self {
            Self::Preview => 4,
            Self::Final => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationAspect {
    Square,
    Landscape,
    Portrait,
    Cinematic,
}

impl GenerationAspect {
    pub const ALL: [Self; 4] = [
        Self::Square,
        Self::Landscape,
        Self::Portrait,
        Self::Cinematic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Landscape => "Landscape",
            Self::Portrait => "Portrait",
            Self::Cinematic => "Cinematic",
        }
    }

    pub fn dimensions(self, quality: GenerationQuality) -> [u32; 2] {
        match (quality, self) {
            (GenerationQuality::Preview, Self::Square) => [768, 768],
            (GenerationQuality::Preview, Self::Landscape) => [1024, 640],
            (GenerationQuality::Preview, Self::Portrait) => [640, 1024],
            (GenerationQuality::Preview, Self::Cinematic) => [1024, 576],
            (GenerationQuality::Final, Self::Square) => [1024, 1024],
            (GenerationQuality::Final, Self::Landscape) => [1216, 832],
            (GenerationQuality::Final, Self::Portrait) => [832, 1216],
            (GenerationQuality::Final, Self::Cinematic) => [1344, 768],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub id: u64,
    pub prompt: String,
    pub negative_prompt: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub seed: u64,
    pub lora_strength: f32,
    pub output_path: PathBuf,
}

impl GenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt.trim().is_empty() {
            return Err("Describe a scene or create a prompt from live audio.".to_owned());
        }
        if self.prompt.len() + self.negative_prompt.len() > MAX_PROMPT_BYTES {
            return Err("Prompt is too large.".to_owned());
        }
        if ![512, 576, 640, 768, 832, 1024, 1216, 1344].contains(&self.width)
            || ![512, 576, 640, 768, 832, 1024, 1216, 1344].contains(&self.height)
        {
            return Err("Unsupported generation dimensions.".to_owned());
        }
        if !(1..=16).contains(&self.steps)
            || !(1.0..=8.0).contains(&self.cfg_scale)
            || !(0.0..=1.0).contains(&self.lora_strength)
        {
            return Err("Generation settings are outside safe bounds.".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationResult {
    pub request_id: u64,
    pub path: PathBuf,
    pub seed: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug)]
pub struct AutoScenePolicy {
    pub enabled: bool,
    pub cooldown: Duration,
    last_energy: f32,
    last_passage: String,
    last_generation: Option<Instant>,
}

impl Default for AutoScenePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            cooldown: Duration::from_secs(90),
            last_energy: 0.0,
            last_passage: String::new(),
            last_generation: None,
        }
    }
}

impl AutoScenePolicy {
    pub fn observe(
        &mut self,
        studio_active: bool,
        passage: &str,
        energy: f32,
        now: Instant,
    ) -> bool {
        let passage_changed = !self.last_passage.is_empty() && self.last_passage != passage;
        let energy_changed = (energy - self.last_energy).abs() >= 0.25;
        let cooled_down = self
            .last_generation
            .is_none_or(|last| now.duration_since(last) >= self.cooldown);
        self.last_passage = passage.to_owned();
        self.last_energy = energy;
        let trigger =
            self.enabled && studio_active && passage_changed && energy_changed && cooled_down;
        if trigger {
            self.last_generation = Some(now);
        }
        trigger
    }
}

#[derive(Clone, Debug)]
pub enum AiStatus {
    Missing,
    Ready,
    Loading,
    Loaded,
    Generating(u8),
    SceneReady,
    Error(String),
}

impl AiStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Missing => "AI PACK MISSING".to_owned(),
            Self::Ready => "AI READY".to_owned(),
            Self::Loading => "AI LOADING".to_owned(),
            Self::Loaded => "AI READY".to_owned(),
            Self::Generating(progress) => format!("AI GENERATING · {progress}%"),
            Self::SceneReady => "AI SCENE READY".to_owned(),
            Self::Error(_) => "AI ERROR".to_owned(),
        }
    }

    pub fn loaded(&self) -> bool {
        matches!(self, Self::Loaded | Self::Generating(_) | Self::SceneReady)
    }

    pub fn generating(&self) -> bool {
        matches!(self, Self::Generating(_))
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerCommand<'a> {
    Load {
        model: &'a Path,
        lora: Option<&'a Path>,
    },
    Generate {
        request: &'a GenerationRequest,
    },
    Cancel,
    Unload,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerEvent {
    Ready,
    Progress { request_id: u64, percent: u8 },
    Result { result: GenerationResult },
    Error { message: String },
}

enum SessionCommand {
    Load(PathBuf),
    Generate(GenerationRequest),
    Cancel,
    Unload,
    Shutdown,
}

pub struct LocalAiSession {
    commands: Sender<SessionCommand>,
    events: Receiver<WorkerEvent>,
    pub status: AiStatus,
    pub pack_root: Option<PathBuf>,
    pub history: VecDeque<GenerationResult>,
    pending: Option<GenerationRequest>,
    active_since: Option<(Instant, Duration)>,
}

impl Default for LocalAiSession {
    fn default() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        thread::spawn(move || session_thread(command_rx, event_tx));
        let pack_root = discover_ai_pack();
        Self {
            commands: command_tx,
            events: event_rx,
            status: if pack_root.is_some() {
                AiStatus::Ready
            } else {
                AiStatus::Missing
            },
            pack_root,
            history: VecDeque::new(),
            pending: None,
            active_since: None,
        }
    }
}

impl LocalAiSession {
    pub fn locate(&mut self, root: PathBuf) {
        self.pack_root = Some(root);
        self.status = AiStatus::Ready;
    }

    pub fn load(&mut self) {
        let Some(root) = self.pack_root.clone() else {
            self.status = AiStatus::Missing;
            return;
        };
        self.status = AiStatus::Loading;
        if self.commands.send(SessionCommand::Load(root)).is_err() {
            self.status = AiStatus::Error("AI worker thread stopped.".to_owned());
        }
    }

    pub fn generate(&mut self, request: GenerationRequest) -> Result<(), String> {
        request.validate()?;
        if matches!(self.status, AiStatus::Ready) {
            self.pending = Some(request);
            self.load();
            return Ok(());
        }
        if self.status.generating() {
            if self.pending.is_some() {
                return Err("One scene is active and one is already queued.".to_owned());
            }
            self.pending = Some(request);
            return Ok(());
        }
        if !self.status.loaded() {
            return Err("Load the local AI Pack first.".to_owned());
        }
        self.active_since = Some((
            Instant::now(),
            if request.steps <= 4 {
                GenerationQuality::Preview.timeout()
            } else {
                GenerationQuality::Final.timeout()
            },
        ));
        self.status = AiStatus::Generating(0);
        self.commands
            .send(SessionCommand::Generate(request))
            .map_err(|_| "AI worker thread stopped.".to_owned())
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.active_since = None;
        let _ = self.commands.send(SessionCommand::Cancel);
        self.status = AiStatus::Loading;
    }

    pub fn unload(&mut self) {
        self.pending = None;
        self.active_since = None;
        let _ = self.commands.send(SessionCommand::Unload);
        self.status = if self.pack_root.is_some() {
            AiStatus::Ready
        } else {
            AiStatus::Missing
        };
    }

    pub fn poll(&mut self) -> Vec<GenerationResult> {
        if self
            .active_since
            .is_some_and(|(started, timeout)| started.elapsed() >= timeout)
        {
            self.cancel();
            self.status =
                AiStatus::Error("Generation timed out; the worker is restarting.".to_owned());
        }
        let mut results = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            match event {
                WorkerEvent::Ready => {
                    if let Some(next) = self.pending.take() {
                        self.active_since = Some((
                            Instant::now(),
                            if next.steps <= 4 {
                                GenerationQuality::Preview.timeout()
                            } else {
                                GenerationQuality::Final.timeout()
                            },
                        ));
                        self.status = AiStatus::Generating(0);
                        let _ = self.commands.send(SessionCommand::Generate(next));
                    } else {
                        self.status = AiStatus::Loaded;
                    }
                }
                WorkerEvent::Progress {
                    request_id,
                    percent,
                } => {
                    self.status = if request_id == 0 {
                        AiStatus::Loading
                    } else {
                        AiStatus::Generating(percent.min(100))
                    };
                }
                WorkerEvent::Result { result } => {
                    self.active_since = None;
                    self.history.push_front(result.clone());
                    self.history.truncate(MAX_HISTORY);
                    results.push(result);
                    if let Some(next) = self.pending.take() {
                        self.active_since = Some((
                            Instant::now(),
                            if next.steps <= 4 {
                                GenerationQuality::Preview.timeout()
                            } else {
                                GenerationQuality::Final.timeout()
                            },
                        ));
                        self.status = AiStatus::Generating(0);
                        let _ = self.commands.send(SessionCommand::Generate(next));
                    } else {
                        self.status = AiStatus::SceneReady;
                    }
                }
                WorkerEvent::Error { message } => self.status = AiStatus::Error(message),
            }
        }
        results
    }
}

impl Drop for LocalAiSession {
    fn drop(&mut self) {
        let _ = self.commands.send(SessionCommand::Shutdown);
    }
}

fn discover_ai_pack() -> Option<PathBuf> {
    let mut candidates = std::env::var_os("THEVISUALIZER_AI_PACK")
        .map(PathBuf::from)
        .into_iter()
        .chain(
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("ai-pack"))),
        );
    candidates.find(|root| root.join("manifest.json").is_file())
}

fn read_manifest(root: &Path) -> Result<(AiPackManifest, PathBuf), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("Cannot open AI Pack: {error}"))?;
    let bytes =
        fs::read(root.join("manifest.json")).map_err(|error| format!("Manifest: {error}"))?;
    if bytes.len() > 128 * 1024 {
        return Err("AI Pack manifest is too large.".to_owned());
    }
    let manifest: AiPackManifest =
        serde_json::from_slice(&bytes).map_err(|error| format!("Manifest: {error}"))?;
    if manifest.format != 1 || manifest.runner_commit != RUNNER_COMMIT {
        return Err("AI Pack version does not match this application.".to_owned());
    }
    if !manifest.model.sha256.eq_ignore_ascii_case(MODEL_SHA256) {
        return Err("AI Pack does not contain the approved SFW checkpoint.".to_owned());
    }
    Ok((manifest, root))
}

fn checked_file(root: &Path, file: &PackFile) -> Result<PathBuf, String> {
    let path = root
        .join(&file.path)
        .canonicalize()
        .map_err(|error| format!("Missing {}: {error}", file.path.display()))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(format!("Unsafe AI Pack path: {}", file.path.display()));
    }
    let actual = sha256(&path)?;
    if !actual.eq_ignore_ascii_case(&file.sha256) {
        return Err(format!("Checksum failed: {}", file.path.display()));
    }
    Ok(path)
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn session_thread(commands: Receiver<SessionCommand>, events: Sender<WorkerEvent>) {
    let mut worker: Option<(Child, ChildStdin)> = None;
    let mut loaded_root: Option<PathBuf> = None;
    while let Ok(command) = commands.recv() {
        let result = match command {
            SessionCommand::Load(root) => {
                stop_worker(&mut worker);
                start_worker(&root, &events).map(|running| {
                    loaded_root = Some(root);
                    worker = Some(running);
                })
            }
            SessionCommand::Generate(request) => {
                send_worker(&mut worker, &WorkerCommand::Generate { request: &request })
            }
            SessionCommand::Cancel => {
                let _ = send_worker(&mut worker, &WorkerCommand::Cancel);
                stop_worker(&mut worker);
                loaded_root
                    .as_ref()
                    .ok_or_else(|| "AI Pack is not loaded.".to_owned())
                    .and_then(|root| start_worker(root, &events))
                    .map(|running| worker = Some(running))
            }
            SessionCommand::Unload => {
                let _ = send_worker(&mut worker, &WorkerCommand::Unload);
                stop_worker(&mut worker);
                loaded_root = None;
                Ok(())
            }
            SessionCommand::Shutdown => {
                stop_worker(&mut worker);
                break;
            }
        };
        if let Err(message) = result {
            let _ = events.send(WorkerEvent::Error { message });
        }
    }
}

fn start_worker(root: &Path, events: &Sender<WorkerEvent>) -> Result<(Child, ChildStdin), String> {
    let (manifest, root) = read_manifest(root)?;
    let model = checked_file(&root, &manifest.model)?;
    let lora = manifest
        .lora
        .as_ref()
        .map(|file| checked_file(&root, file))
        .transpose()?;
    for runtime in &manifest.runtime {
        checked_file(&root, runtime)?;
    }
    let worker_file = manifest
        .workers
        .cuda
        .as_ref()
        .or(manifest.workers.vulkan.as_ref())
        .ok_or_else(|| "This AI Pack has no CUDA or Vulkan worker.".to_owned())?;
    let executable = checked_file(&root, worker_file)?;
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not launch the local AI worker: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "AI worker stdin is unavailable.".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "AI worker stdout is unavailable.".to_owned())?;
    let event_tx = events.clone();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            match serde_json::from_str::<WorkerEvent>(&line) {
                Ok(event) => {
                    let _ = event_tx.send(event);
                }
                Err(error) => {
                    let _ = event_tx.send(WorkerEvent::Error {
                        message: format!("Invalid AI worker response: {error}"),
                    });
                }
            }
        }
    });
    let mut running = Some((child, stdin));
    send_worker(
        &mut running,
        &WorkerCommand::Load {
            model: &model,
            lora: lora.as_deref(),
        },
    )?;
    Ok(running.expect("worker remains present after load"))
}

fn send_worker<T: Serialize>(
    worker: &mut Option<(Child, ChildStdin)>,
    command: &T,
) -> Result<(), String> {
    let worker = worker
        .as_mut()
        .ok_or_else(|| "Load the AI Pack first.".to_owned())?;
    serde_json::to_writer(&mut worker.1, command).map_err(|error| error.to_string())?;
    worker
        .1
        .write_all(b"\n")
        .map_err(|error| error.to_string())?;
    worker.1.flush().map_err(|error| error.to_string())
}

fn stop_worker(worker: &mut Option<(Child, ChildStdin)>) {
    if let Some((mut child, _)) = worker.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn generated_directory() -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is unavailable.".to_owned())?;
    let directory = base.join("TheVisualizer").join("generated");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory)
}

pub fn write_metadata(
    result: &GenerationResult,
    request: &GenerationRequest,
) -> Result<PathBuf, String> {
    let path = result.path.with_extension("json");
    let bytes = serde_json::to_vec_pretty(request).map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_profiles_are_bounded() {
        for quality in [GenerationQuality::Preview, GenerationQuality::Final] {
            for aspect in GenerationAspect::ALL {
                let [width, height] = aspect.dimensions(quality);
                let request = GenerationRequest {
                    id: 1,
                    prompt: "tvizfield luminous orbital garden".to_owned(),
                    negative_prompt: String::new(),
                    width,
                    height,
                    steps: quality.steps(),
                    cfg_scale: 2.0,
                    seed: 7,
                    lora_strength: 0.65,
                    output_path: PathBuf::from("scene.png"),
                };
                assert!(request.validate().is_ok());
            }
        }
    }

    #[test]
    fn auto_scene_requires_all_gates() {
        let now = Instant::now();
        let mut policy = AutoScenePolicy {
            enabled: true,
            ..Default::default()
        };
        assert!(!policy.observe(true, "quiet", 0.1, now));
        assert!(!policy.observe(false, "drop", 0.8, now + Duration::from_secs(1)));
        assert!(policy.observe(true, "quiet", 0.1, now + Duration::from_secs(2)));
        assert!(!policy.observe(true, "drop", 0.8, now + Duration::from_secs(3)));
        assert!(policy.observe(true, "quiet", 0.1, now + Duration::from_secs(93)));
    }
}
