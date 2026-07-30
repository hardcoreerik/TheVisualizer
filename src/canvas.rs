use std::collections::VecDeque;

use eframe::egui::{Pos2, Rect};

pub const MAX_CANVAS_FORMS: usize = 32;
pub const MAX_FORM_POINTS: usize = 64;
const MAX_HISTORY: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasTool {
    Select,
    Brush,
    Pen,
    Line,
    Rectangle,
    Ellipse,
    Eraser,
}

impl CanvasTool {
    pub const ALL: [Self; 7] = [
        Self::Select,
        Self::Brush,
        Self::Pen,
        Self::Line,
        Self::Rectangle,
        Self::Ellipse,
        Self::Eraser,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select [V]",
            Self::Brush => "Brush [B]",
            Self::Pen => "Pen [P]",
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Eraser => "Eraser [E]",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasFormKind {
    Polygon,
    Ribbon,
    Beam,
    Rectangle,
    Ellipse,
}

impl CanvasFormKind {
    pub const ALL: [Self; 5] = [
        Self::Polygon,
        Self::Ribbon,
        Self::Beam,
        Self::Rectangle,
        Self::Ellipse,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Polygon => "Filled Form",
            Self::Ribbon => "Reactive Ribbon",
            Self::Beam => "Signal Beam",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse / Orbit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasContent {
    Solid,
    Gradient,
    Waveform,
    Spectrum,
    Particles,
    Pulse,
    Grid,
    Halo,
    Orbits,
}

impl CanvasContent {
    pub const ALL: [Self; 9] = [
        Self::Solid,
        Self::Gradient,
        Self::Waveform,
        Self::Spectrum,
        Self::Particles,
        Self::Pulse,
        Self::Grid,
        Self::Halo,
        Self::Orbits,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Material",
            Self::Gradient => "Gradient",
            Self::Waveform => "Waveform",
            Self::Spectrum => "Spectrum",
            Self::Particles => "Particles",
            Self::Pulse => "Pulse Trace",
            Self::Grid => "Cyber Grid",
            Self::Halo => "Halo Spectrum",
            Self::Orbits => "Atomic Orbits",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasBand {
    Full,
    Bass,
    Mid,
    Treble,
    Onset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasBackground {
    Preset,
    NeonScope,
    ParticleForge,
    Studio,
}

impl CanvasBackground {
    pub const ALL: [Self; 4] = [
        Self::Preset,
        Self::NeonScope,
        Self::ParticleForge,
        Self::Studio,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Preset => "Visual Library Preset",
            Self::NeonScope => "Neon Scope",
            Self::ParticleForge => "Particle Forge",
            Self::Studio => "Studio Composition",
        }
    }
}

impl CanvasBand {
    pub const ALL: [Self; 5] = [Self::Full, Self::Bass, Self::Mid, Self::Treble, Self::Onset];

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Bass => "Bass",
            Self::Mid => "Mid",
            Self::Treble => "Treble",
            Self::Onset => "Onset",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasForm {
    pub kind: CanvasFormKind,
    pub content: CanvasContent,
    pub band: CanvasBand,
    pub points: Vec<[f32; 2]>,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub reactivity: f32,
    pub stroke_width: f32,
    pub scale: f32,
    pub rotation: f32,
    pub filled: bool,
}

impl CanvasForm {
    fn new(kind: CanvasFormKind, points: Vec<[f32; 2]>) -> Self {
        Self {
            kind,
            content: match kind {
                CanvasFormKind::Ribbon | CanvasFormKind::Beam => CanvasContent::Waveform,
                CanvasFormKind::Ellipse => CanvasContent::Halo,
                _ => CanvasContent::Gradient,
            },
            band: CanvasBand::Full,
            points,
            visible: true,
            locked: false,
            opacity: 0.9,
            reactivity: 1.0,
            stroke_width: 3.0,
            scale: 1.0,
            rotation: 0.0,
            filled: !matches!(kind, CanvasFormKind::Ribbon | CanvasFormKind::Beam),
        }
    }

    pub fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        let first = *self.points.first()?;
        let mut min = first;
        let mut max = first;
        for point in &self.points[1..] {
            min[0] = min[0].min(point[0]);
            min[1] = min[1].min(point[1]);
            max[0] = max[0].max(point[0]);
            max[1] = max[1].max(point[1]);
        }
        Some((min, max))
    }
}

pub struct CanvasState {
    pub forms: Vec<CanvasForm>,
    pub selected: Option<usize>,
    pub tool: CanvasTool,
    pub draft: Vec<[f32; 2]>,
    pub show_background: bool,
    pub background: CanvasBackground,
    pub background_opacity: f32,
    pub show_handles: bool,
    pub notice: Option<String>,
    drag_start: Option<[f32; 2]>,
    drag_points: Vec<[f32; 2]>,
    undo: VecDeque<Vec<CanvasForm>>,
    redo: VecDeque<Vec<CanvasForm>>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            forms: Vec::new(),
            selected: None,
            tool: CanvasTool::Brush,
            draft: Vec::new(),
            show_background: true,
            background: CanvasBackground::Preset,
            background_opacity: 0.32,
            show_handles: true,
            notice: None,
            drag_start: None,
            drag_points: Vec::new(),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
        }
    }
}

impl CanvasState {
    fn remember(&mut self) {
        if self.undo.len() == MAX_HISTORY {
            self.undo.pop_front();
        }
        self.undo.push_back(self.forms.clone());
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some(forms) = self.undo.pop_back() {
            self.redo
                .push_back(std::mem::replace(&mut self.forms, forms));
            self.selected = self.selected.filter(|index| *index < self.forms.len());
        }
    }

    pub fn redo(&mut self) {
        if let Some(forms) = self.redo.pop_back() {
            self.undo
                .push_back(std::mem::replace(&mut self.forms, forms));
            self.selected = self.selected.filter(|index| *index < self.forms.len());
        }
    }

    pub fn clear(&mut self) {
        if !self.forms.is_empty() {
            self.remember();
            self.forms.clear();
            self.selected = None;
        }
    }

    pub fn remove_selected(&mut self) {
        if let Some(index) = self.selected.filter(|index| *index < self.forms.len()) {
            self.remember();
            self.forms.remove(index);
            self.selected = (index < self.forms.len()).then_some(index);
        }
    }

    pub fn duplicate_selected(&mut self) {
        let Some(index) = self.selected.filter(|index| *index < self.forms.len()) else {
            return;
        };
        if self.forms.len() == MAX_CANVAS_FORMS {
            self.notice = Some(format!("Canvas is limited to {MAX_CANVAS_FORMS} forms"));
            return;
        }
        self.remember();
        let mut form = self.forms[index].clone();
        for point in &mut form.points {
            point[0] = (point[0] + 0.025).clamp(0.0, 1.0);
            point[1] = (point[1] + 0.025).clamp(0.0, 1.0);
        }
        self.forms.push(form);
        self.selected = Some(self.forms.len() - 1);
    }

    pub fn move_selected_layer(&mut self, offset: isize) {
        let Some(index) = self.selected.filter(|index| *index < self.forms.len()) else {
            return;
        };
        let target = index
            .saturating_add_signed(offset)
            .min(self.forms.len().saturating_sub(1));
        if target != index {
            self.remember();
            self.forms.swap(index, target);
            self.selected = Some(target);
        }
    }

    pub fn begin(&mut self, point: [f32; 2]) {
        self.draft.clear();
        self.drag_start = Some(point);
        if self.tool == CanvasTool::Select {
            self.selected = self.nearest(point, 0.045);
            self.drag_points = self
                .selected
                .and_then(|index| self.forms.get(index))
                .filter(|form| !form.locked)
                .map_or_else(Vec::new, |form| form.points.clone());
        } else if self.tool == CanvasTool::Eraser {
            self.selected = self.nearest(point, 0.055);
            self.remove_selected();
        } else {
            self.draft.push(point);
        }
    }

    pub fn update(&mut self, point: [f32; 2]) {
        if self.tool == CanvasTool::Select {
            let Some(start) = self.drag_start else { return };
            let Some(index) = self.selected.filter(|index| *index < self.forms.len()) else {
                return;
            };
            if self.forms[index].locked || self.drag_points.is_empty() {
                return;
            }
            let delta = [point[0] - start[0], point[1] - start[1]];
            for (target, original) in self.forms[index].points.iter_mut().zip(&self.drag_points) {
                target[0] = (original[0] + delta[0]).clamp(0.0, 1.0);
                target[1] = (original[1] + delta[1]).clamp(0.0, 1.0);
            }
        } else if self.tool != CanvasTool::Eraser {
            let replace_end = matches!(
                self.tool,
                CanvasTool::Line | CanvasTool::Rectangle | CanvasTool::Ellipse
            );
            if replace_end && self.draft.len() > 1 {
                self.draft[1] = point;
            } else if self.draft.len() < MAX_FORM_POINTS
                && self
                    .draft
                    .last()
                    .is_none_or(|last| distance(*last, point) > 0.006)
            {
                self.draft.push(point);
            }
        }
    }

    pub fn finish(&mut self, point: [f32; 2]) {
        if self.tool == CanvasTool::Select {
            if self.selected.is_some()
                && !self.drag_points.is_empty()
                && self.drag_points != self.selected_points()
            {
                let before = std::mem::take(&mut self.drag_points);
                let current = self.forms.clone();
                if let Some(index) = self.selected {
                    let mut previous = current.clone();
                    previous[index].points = before;
                    if self.undo.len() == MAX_HISTORY {
                        self.undo.pop_front();
                    }
                    self.undo.push_back(previous);
                    self.redo.clear();
                }
            }
            self.drag_start = None;
            return;
        }
        if self.tool == CanvasTool::Eraser {
            self.drag_start = None;
            return;
        }
        self.update(point);
        if self.forms.len() == MAX_CANVAS_FORMS {
            self.notice = Some(format!("Canvas is limited to {MAX_CANVAS_FORMS} forms"));
            self.draft.clear();
            return;
        }
        let dot = self.draft.len() <= 2
            && self
                .draft
                .first()
                .zip(self.draft.last())
                .is_some_and(|(first, last)| distance(*first, *last) < 0.004);
        let points = if dot {
            let center = self.draft[0];
            vec![
                [
                    (center[0] - 0.025).clamp(0.0, 1.0),
                    (center[1] - 0.025).clamp(0.0, 1.0),
                ],
                [
                    (center[0] + 0.025).clamp(0.0, 1.0),
                    (center[1] + 0.025).clamp(0.0, 1.0),
                ],
            ]
        } else {
            simplify(&self.draft)
        };
        if let Some(kind) = dot
            .then_some(CanvasFormKind::Ellipse)
            .or_else(|| recognize(self.tool, &points))
        {
            self.remember();
            let mut form = CanvasForm::new(kind, points);
            if dot {
                form.content = CanvasContent::Particles;
                form.band = CanvasBand::Onset;
            }
            self.forms.push(form);
            self.selected = Some(self.forms.len() - 1);
            self.tool = CanvasTool::Select;
        }
        self.draft.clear();
        self.drag_start = None;
    }

    fn selected_points(&self) -> Vec<[f32; 2]> {
        self.selected
            .and_then(|index| self.forms.get(index))
            .map_or_else(Vec::new, |form| form.points.clone())
    }

    pub fn nearest(&self, point: [f32; 2], tolerance: f32) -> Option<usize> {
        self.forms
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, form)| form.visible)
            .find_map(|(index, form)| {
                let within_bounds = form.bounds().is_some_and(|(min, max)| {
                    point[0] >= min[0] - tolerance
                        && point[0] <= max[0] + tolerance
                        && point[1] >= min[1] - tolerance
                        && point[1] <= max[1] + tolerance
                });
                (within_bounds
                    || form
                        .points
                        .iter()
                        .any(|candidate| distance(*candidate, point) <= tolerance))
                .then_some(index)
            })
    }

    pub fn select_at(&mut self, point: [f32; 2]) {
        self.selected = self.nearest(point, 0.05);
    }

    pub fn normalize(rect: Rect, point: Pos2) -> [f32; 2] {
        [
            ((point.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0),
            ((point.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0),
        ]
    }

    pub fn screen(rect: Rect, point: [f32; 2]) -> Pos2 {
        Pos2::new(
            rect.left() + point[0] * rect.width(),
            rect.top() + point[1] * rect.height(),
        )
    }

    pub fn encode_scene(&self) -> String {
        let mut source = format!(
            "canvas=1\nbackground={},{},{}\n",
            u8::from(self.show_background),
            self.background as u8,
            self.background_opacity
        );
        for form in &self.forms {
            let points = form
                .points
                .iter()
                .map(|point| format!("{:.5}:{:.5}", point[0], point[1]))
                .collect::<Vec<_>>()
                .join(";");
            source.push_str(&format!(
                "form={},{},{},{},{},{},{},{},{},{},{},{}|{}\n",
                form.kind as u8,
                form.content as u8,
                form.band as u8,
                u8::from(form.visible),
                u8::from(form.locked),
                form.opacity,
                form.reactivity,
                form.stroke_width,
                form.scale,
                form.rotation,
                u8::from(form.filled),
                form.points.len(),
                points
            ));
        }
        source
    }

    pub fn decode_scene(source: &str) -> Result<Self, String> {
        let mut state = Self::default();
        let mut header = false;
        for line in source.lines() {
            if line == "canvas=1" {
                header = true;
            } else if let Some(value) = line.strip_prefix("background=") {
                let mut values = value.split(',');
                state.show_background = parse_u8(values.next())? != 0;
                state.background = CanvasBackground::ALL
                    .get(parse_usize(values.next())?)
                    .copied()
                    .ok_or_else(|| "Canvas background is invalid".to_owned())?;
                state.background_opacity = parse_f32(values.next(), 0.0, 1.0)?;
            } else if let Some(value) = line.strip_prefix("form=") {
                if state.forms.len() == MAX_CANVAS_FORMS {
                    return Err(format!("Canvas exceeds {MAX_CANVAS_FORMS} forms"));
                }
                let (metadata, point_source) = value
                    .split_once('|')
                    .ok_or_else(|| "Canvas form is missing points".to_owned())?;
                let values = metadata.split(',').collect::<Vec<_>>();
                if values.len() != 12 {
                    return Err("Canvas form metadata is incomplete".to_owned());
                }
                let kind = CanvasFormKind::ALL
                    .get(parse_usize(Some(values[0]))?)
                    .copied()
                    .ok_or_else(|| "Canvas form kind is invalid".to_owned())?;
                let content = CanvasContent::ALL
                    .get(parse_usize(Some(values[1]))?)
                    .copied()
                    .ok_or_else(|| "Canvas content is invalid".to_owned())?;
                let band = CanvasBand::ALL
                    .get(parse_usize(Some(values[2]))?)
                    .copied()
                    .ok_or_else(|| "Canvas band is invalid".to_owned())?;
                let point_count = parse_usize(Some(values[11]))?;
                if !(2..=MAX_FORM_POINTS).contains(&point_count) {
                    return Err("Canvas point count is invalid".to_owned());
                }
                let points = point_source
                    .split(';')
                    .map(|pair| {
                        let (x, y) = pair
                            .split_once(':')
                            .ok_or_else(|| "Canvas point is malformed".to_owned())?;
                        Ok([parse_f32(Some(x), 0.0, 1.0)?, parse_f32(Some(y), 0.0, 1.0)?])
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                if points.len() != point_count {
                    return Err("Canvas point count does not match".to_owned());
                }
                state.forms.push(CanvasForm {
                    kind,
                    content,
                    band,
                    points,
                    visible: parse_u8(Some(values[3]))? != 0,
                    locked: parse_u8(Some(values[4]))? != 0,
                    opacity: parse_f32(Some(values[5]), 0.0, 1.0)?,
                    reactivity: parse_f32(Some(values[6]), 0.0, 3.0)?,
                    stroke_width: parse_f32(Some(values[7]), 0.5, 16.0)?,
                    scale: parse_f32(Some(values[8]), 0.2, 3.0)?,
                    rotation: parse_f32(
                        Some(values[9]),
                        -std::f32::consts::TAU,
                        std::f32::consts::TAU,
                    )?,
                    filled: parse_u8(Some(values[10]))? != 0,
                });
            }
        }
        if !header {
            return Err("Canvas scene header is missing".to_owned());
        }
        state.selected = (!state.forms.is_empty()).then_some(0);
        Ok(state)
    }
}

pub fn transformed_points(form: &CanvasForm, rect: Rect, audio_scale: f32) -> Vec<Pos2> {
    let Some((min, max)) = form.bounds() else {
        return Vec::new();
    };
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let scale = form.scale * audio_scale;
    let cosine = form.rotation.cos();
    let sine = form.rotation.sin();
    form.points
        .iter()
        .map(|point| {
            let x = (point[0] - center[0]) * scale;
            let y = (point[1] - center[1]) * scale;
            CanvasState::screen(
                rect,
                [
                    center[0] + x * cosine - y * sine,
                    center[1] + x * sine + y * cosine,
                ],
            )
        })
        .collect()
}

fn recognize(tool: CanvasTool, points: &[[f32; 2]]) -> Option<CanvasFormKind> {
    if points.len() < 2 {
        return None;
    }
    match tool {
        CanvasTool::Line => Some(CanvasFormKind::Beam),
        CanvasTool::Rectangle => Some(CanvasFormKind::Rectangle),
        CanvasTool::Ellipse => Some(CanvasFormKind::Ellipse),
        CanvasTool::Brush | CanvasTool::Pen => {
            if points.len() >= 4 && distance(points[0], *points.last()?) < 0.075 {
                let (min, max) = point_bounds(points);
                let width = max[0] - min[0];
                let height = max[1] - min[1];
                let aspect = width / height.max(0.001);
                if (0.72..=1.38).contains(&aspect) {
                    Some(CanvasFormKind::Ellipse)
                } else {
                    Some(CanvasFormKind::Polygon)
                }
            } else if line_deviation(points) < 0.018 {
                Some(CanvasFormKind::Beam)
            } else {
                Some(CanvasFormKind::Ribbon)
            }
        }
        CanvasTool::Select | CanvasTool::Eraser => None,
    }
}

fn simplify(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut result = Vec::new();
    for point in points {
        if result
            .last()
            .is_none_or(|previous| distance(*previous, *point) > 0.009)
        {
            result.push(*point);
        }
        if result.len() == MAX_FORM_POINTS {
            break;
        }
    }
    if result.len() == 1 {
        result.push(result[0]);
    }
    result
}

fn point_bounds(points: &[[f32; 2]]) -> ([f32; 2], [f32; 2]) {
    let mut min = points[0];
    let mut max = points[0];
    for point in &points[1..] {
        min[0] = min[0].min(point[0]);
        min[1] = min[1].min(point[1]);
        max[0] = max[0].max(point[0]);
        max[1] = max[1].max(point[1]);
    }
    (min, max)
}

fn line_deviation(points: &[[f32; 2]]) -> f32 {
    let start = points[0];
    let end = points[points.len() - 1];
    let direction = [end[0] - start[0], end[1] - start[1]];
    let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
    if length < 0.001 {
        return f32::MAX;
    }
    points
        .iter()
        .map(|point| {
            ((point[0] - start[0]) * direction[1] - (point[1] - start[1]) * direction[0]).abs()
                / length
        })
        .fold(0.0, f32::max)
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn parse_u8(value: Option<&str>) -> Result<u8, String> {
    value
        .ok_or_else(|| "Canvas value is missing".to_owned())?
        .parse()
        .map_err(|_| "Canvas integer is invalid".to_owned())
}

fn parse_usize(value: Option<&str>) -> Result<usize, String> {
    value
        .ok_or_else(|| "Canvas value is missing".to_owned())?
        .parse()
        .map_err(|_| "Canvas integer is invalid".to_owned())
}

fn parse_f32(value: Option<&str>, minimum: f32, maximum: f32) -> Result<f32, String> {
    let value = value
        .ok_or_else(|| "Canvas value is missing".to_owned())?
        .parse::<f32>()
        .map_err(|_| "Canvas number is invalid".to_owned())?;
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err("Canvas number is out of range".to_owned());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_recognizes_forms_and_round_trips_bounded_state() {
        let closed = [[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8], [0.21, 0.21]];
        assert_eq!(
            recognize(CanvasTool::Brush, &closed),
            Some(CanvasFormKind::Ellipse)
        );
        let open = [[0.1, 0.2], [0.3, 0.5], [0.6, 0.4], [0.9, 0.7]];
        assert_eq!(
            recognize(CanvasTool::Brush, &open),
            Some(CanvasFormKind::Ribbon)
        );

        let mut canvas = CanvasState::default();
        canvas
            .forms
            .push(CanvasForm::new(CanvasFormKind::Polygon, closed.to_vec()));
        canvas.forms[0].content = CanvasContent::Particles;
        canvas.forms[0].band = CanvasBand::Bass;
        let decoded = CanvasState::decode_scene(&canvas.encode_scene()).unwrap();
        assert_eq!(decoded.forms, canvas.forms);
    }
}
