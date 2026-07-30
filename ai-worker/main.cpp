#include "stable-diffusion.h"
#include "json.hpp"

#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "stb_image_write.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <filesystem>
#include <iostream>
#include <memory>
#include <mutex>
#include <string>
#include <thread>

using json = nlohmann::json;
namespace fs = std::filesystem;

namespace {
std::mutex output_mutex;
std::mutex generation_mutex;
std::atomic<uint64_t> active_request{0};
std::atomic<bool> generating{false};
sd_ctx_t* context = nullptr;
std::thread generation_thread;
std::string lora_path;

void emit(const json& value) {
    std::lock_guard lock(output_mutex);
    std::cout << value.dump() << '\n' << std::flush;
}

void log_to_stderr(sd_log_level_t, const char* text, void*) {
    std::cerr << text;
}

void progress(int step, int steps, float, void*) {
    const int percent = steps > 0 ? std::clamp(step * 100 / steps, 0, 100) : 0;
    emit({{"type", "progress"}, {"request_id", active_request.load()}, {"percent", percent}});
}

void join_generation() {
    if (generation_thread.joinable()) {
        generation_thread.join();
    }
}

bool safe_output(const fs::path& path) {
    std::error_code error;
    const auto parent = fs::weakly_canonical(path.parent_path(), error);
    return !error && fs::exists(parent) && path.extension() == ".png" && path.is_absolute();
}

void generate(json request) {
    const auto started = std::chrono::steady_clock::now();
    try {
        const uint64_t id = request.at("id").get<uint64_t>();
        const std::string prompt = request.at("prompt").get<std::string>();
        const std::string negative = request.value("negative_prompt", "");
        const int width = request.at("width").get<int>();
        const int height = request.at("height").get<int>();
        const int steps = request.at("steps").get<int>();
        const float cfg = request.at("cfg_scale").get<float>();
        const int64_t seed = request.at("seed").get<int64_t>();
        const float lora_strength = request.value("lora_strength", 0.0f);
        const fs::path output = request.at("output_path").get<std::string>();
        if (prompt.empty() || prompt.size() + negative.size() > 16 * 1024 ||
            width < 512 || width > 1344 || height < 512 || height > 1344 ||
            steps < 1 || steps > 16 || cfg < 1.0f || cfg > 8.0f ||
            !safe_output(output)) {
            throw std::runtime_error("generation request is outside safe bounds");
        }

        sd_img_gen_params_t params{};
        sd_img_gen_params_init(&params);
        params.prompt = prompt.c_str();
        params.negative_prompt = negative.c_str();
        params.width = width;
        params.height = height;
        params.sample_params.sample_method = EULER_A_SAMPLE_METHOD;
        params.sample_params.sample_steps = steps;
        params.sample_params.guidance.txt_cfg = cfg;
        params.seed = seed;
        params.batch_count = 1;
        sd_lora_t lora{};
        if (!lora_path.empty() && lora_strength > 0.0f) {
            lora.path = lora_path.c_str();
            lora.multiplier = lora_strength;
            params.loras = &lora;
            params.lora_count = 1;
        }

        sd_image_t* images = nullptr;
        int count = 0;
        if (!generate_image(context, &params, &images, &count) || !images || count < 1) {
            throw std::runtime_error("stable-diffusion.cpp did not produce an image");
        }
        const auto& image = images[0];
        const bool wrote = stbi_write_png(
            output.string().c_str(),
            static_cast<int>(image.width),
            static_cast<int>(image.height),
            static_cast<int>(image.channel),
            image.data,
            static_cast<int>(image.width * image.channel)) != 0;
        free_sd_images(images, count);
        if (!wrote) {
            throw std::runtime_error("could not write generated PNG");
        }
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - started);
        emit({
            {"type", "result"},
            {"result", {
                {"request_id", id},
                {"path", output.string()},
                {"seed", static_cast<uint64_t>(seed)},
                {"elapsed_ms", elapsed.count()}
            }}
        });
    } catch (const std::exception& error) {
        emit({{"type", "error"}, {"message", error.what()}});
    }
    active_request = 0;
    generating = false;
}

void unload() {
    if (generating && context) {
        sd_cancel_generation(context, SD_CANCEL_ALL);
    }
    join_generation();
    if (context) {
        free_sd_ctx(context);
        context = nullptr;
    }
    lora_path.clear();
}
} // namespace

int main() {
    sd_set_log_callback(log_to_stderr, nullptr);
    sd_set_progress_callback(progress, nullptr);
    std::string line;
    while (std::getline(std::cin, line)) {
        try {
            if (line.size() > 64 * 1024) {
                throw std::runtime_error("command is too large");
            }
            const auto command = json::parse(line);
            const auto type = command.at("type").get<std::string>();
            if (type == "load") {
                unload();
                const fs::path model = command.at("model").get<std::string>();
                if (!fs::is_regular_file(model)) {
                    throw std::runtime_error("model file is unavailable");
                }
                lora_path = command.contains("lora") && command["lora"].is_string()
                    ? command["lora"].get<std::string>()
                    : "";
                sd_ctx_params_t params{};
                sd_ctx_params_init(&params);
                const auto model_string = model.string();
                params.model_path = model_string.c_str();
                params.n_threads = std::max(1, sd_get_num_physical_cores() / 2);
                params.flash_attn = true;
                params.diffusion_flash_attn = true;
                params.enable_mmap = true;
                params.eager_load = true;
                context = new_sd_ctx(&params);
                if (!context || !sd_ctx_supports_image_generation(context)) {
                    unload();
                    throw std::runtime_error("model could not be loaded for image generation");
                }
                emit({{"type", "ready"}});
            } else if (type == "generate") {
                if (!context) {
                    throw std::runtime_error("model is not loaded");
                }
                if (generating.exchange(true)) {
                    throw std::runtime_error("generation is already active");
                }
                join_generation();
                const auto request = command.at("request");
                active_request = request.at("id").get<uint64_t>();
                generation_thread = std::thread(generate, request);
            } else if (type == "cancel") {
                if (context && generating) {
                    sd_cancel_generation(context, SD_CANCEL_ALL);
                }
            } else if (type == "unload") {
                unload();
            } else {
                throw std::runtime_error("unknown command");
            }
        } catch (const std::exception& error) {
            emit({{"type", "error"}, {"message", error.what()}});
        }
    }
    unload();
    return 0;
}
