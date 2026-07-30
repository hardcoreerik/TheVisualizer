cmake_minimum_required(VERSION 3.20)
project(thevisualizer_ai_worker LANGUAGES C CXX)

set(SD_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(SD_BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
if(MSVC)
    add_compile_options("$<$<COMPILE_LANGUAGE:CXX>:/bigobj>")
endif()
add_subdirectory("${SDCPP_SOURCE}" stable-diffusion.cpp)

add_executable(thevisualizer-ai-worker main.cpp)
target_compile_features(thevisualizer-ai-worker PRIVATE cxx_std_17)
target_include_directories(thevisualizer-ai-worker PRIVATE
    "${SDCPP_SOURCE}/include"
    "${SDCPP_SOURCE}/thirdparty"
)
target_link_libraries(thevisualizer-ai-worker PRIVATE stable-diffusion)
