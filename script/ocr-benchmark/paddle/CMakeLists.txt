cmake_minimum_required(VERSION 3.16)
project(snaplingo_paddle_bridge LANGUAGES CXX)
set(PADDLE_SDK "/tmp/snaplingo-ocr-benchmark/paddle/sdk-3.3.1/paddle_inference_install_dir" CACHE PATH "Paddle C++ SDK root")
add_library(snaplingo_paddle_bridge SHARED bridge.cpp)
target_compile_features(snaplingo_paddle_bridge PRIVATE cxx_std_17)
target_include_directories(snaplingo_paddle_bridge PRIVATE "${PADDLE_SDK}/paddle/include")
target_link_libraries(snaplingo_paddle_bridge PRIVATE "${PADDLE_SDK}/paddle/lib/libpaddle_inference.dylib")
set_target_properties(snaplingo_paddle_bridge PROPERTIES
  BUILD_RPATH "${PADDLE_SDK}/paddle/lib"
  INSTALL_RPATH "@loader_path")
