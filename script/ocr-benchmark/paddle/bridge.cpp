#include "paddle_inference_api.h"

#include <chrono>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
using Clock = std::chrono::steady_clock;
thread_local std::string last_error;

double elapsed_ms(Clock::time_point start) {
  return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

struct Engine {
  std::shared_ptr<paddle_infer::Predictor> predictor;
  std::string input_name;
  std::string output_name;
  std::vector<int> output_shape;
  std::vector<float> output;
};
}

extern "C" {
const char* sl_paddle_last_error() { return last_error.c_str(); }

const char* sl_paddle_version() {
  static const auto version = paddle_infer::GetVersion();
  return version.c_str();
}

void* sl_paddle_create(const char* model, const char* params, int threads,
                       double* load_ms) {
  try {
    last_error.clear();
    const auto start = Clock::now();
    paddle_infer::Config config;
    config.SetModel(model, params);
    config.DisableGpu();
    config.SetCpuMathLibraryNumThreads(threads);
    config.EnableNewIR();
    config.EnableNewExecutor();
    config.DisableGlogInfo();
    auto engine = std::make_unique<Engine>();
    engine->predictor = paddle_infer::CreatePredictor(config);
    const auto inputs = engine->predictor->GetInputNames();
    const auto outputs = engine->predictor->GetOutputNames();
    if (inputs.size() != 1 || outputs.size() != 1) {
      throw std::runtime_error("OCR bridge requires one input and one output");
    }
    engine->input_name = inputs[0];
    engine->output_name = outputs[0];
    *load_ms = elapsed_ms(start);
    return engine.release();
  } catch (const std::exception& e) {
    last_error = e.what();
    return nullptr;
  }
}

// Timings are: input reshape/copy, predictor Run, output copy (milliseconds).
int sl_paddle_run(void* handle, const float* input, const int* dims, int rank,
                  double* timings) {
  try {
    last_error.clear();
    auto* engine = static_cast<Engine*>(handle);
    auto start = Clock::now();
    auto input_tensor = engine->predictor->GetInputHandle(engine->input_name);
    input_tensor->Reshape(std::vector<int>(dims, dims + rank));
    input_tensor->CopyFromCpu(input);
    timings[0] = elapsed_ms(start);
    start = Clock::now();
    if (!engine->predictor->Run()) {
      throw std::runtime_error("Paddle Predictor::Run returned false");
    }
    timings[1] = elapsed_ms(start);
    start = Clock::now();
    auto output_tensor = engine->predictor->GetOutputHandle(engine->output_name);
    if (output_tensor->type() != paddle_infer::DataType::FLOAT32) {
      throw std::runtime_error("OCR bridge requires a float32 output");
    }
    engine->output_shape = output_tensor->shape();
    size_t count = 1;
    for (int dim : engine->output_shape) count *= dim;
    engine->output.resize(count);
    output_tensor->CopyToCpu(engine->output.data());
    timings[2] = elapsed_ms(start);
    return 0;
  } catch (const std::exception& e) {
    last_error = e.what();
    return -1;
  }
}

int sl_paddle_output_rank(void* handle) {
  return static_cast<Engine*>(handle)->output_shape.size();
}

const int* sl_paddle_output_shape(void* handle) {
  return static_cast<Engine*>(handle)->output_shape.data();
}

const float* sl_paddle_output_data(void* handle) {
  return static_cast<Engine*>(handle)->output.data();
}

void sl_paddle_destroy(void* handle) { delete static_cast<Engine*>(handle); }
}
