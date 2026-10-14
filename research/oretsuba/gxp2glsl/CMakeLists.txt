# Vita3K's shader recompiler alone, to read the eboot's GXP shaders as GLSL:
#
#   cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release \
#       -DVITA3K=<Vita3K checkout> -DGLSLANG=<glslang> -DSPIRV_CROSS=<SPIRV-Cross>
#   ninja -C build && build/gxp2glsl type6.gxp > type6.glsl
#
# A sparse checkout of Vita3K (vita3k/shader, gxm, util, features, config)
# is enough; glslang and SPIRV-Cross at the commits of its
# external/glslang and external/SPIRV-Cross. Needs fmt, spdlog, Boost
# (filesystem) and SDL3 installed.
cmake_minimum_required(VERSION 3.20)
project(gxp2glsl CXX C)
set(CMAKE_CXX_STANDARD 23)
set(V ${VITA3K}/vita3k)
set(SPIRV_CROSS_CLI OFF CACHE BOOL "" FORCE)
set(SPIRV_CROSS_ENABLE_TESTS OFF CACHE BOOL "" FORCE)
add_subdirectory(${SPIRV_CROSS} spirv-cross EXCLUDE_FROM_ALL)
find_package(fmt REQUIRED)
find_package(SDL3 REQUIRED)
find_package(spdlog REQUIRED)
find_package(Boost REQUIRED COMPONENTS filesystem)
file(GLOB SHADER_SRC ${V}/shader/src/*.cpp ${V}/shader/src/translator/*.cpp)
set(G ${GLSLANG}/SPIRV)
add_executable(gxp2glsl main.cpp stubs.cpp ${SHADER_SRC}
  ${V}/gxm/src/gxp.cpp ${V}/gxm/src/attributes.cpp ${V}/gxm/src/color.cpp
  ${V}/util/src/logging.cpp ${V}/util/src/float_to_half.cpp ${V}/util/src/fs_utils.cpp ${V}/util/src/string_utils.cpp
  ${G}/SpvBuilder.cpp ${G}/InReadableOrder.cpp ${G}/Logger.cpp ${G}/disassemble.cpp ${G}/doc.cpp ${G}/SpvPostProcess.cpp)
target_include_directories(gxp2glsl PRIVATE ${V}/shader/include ${V}/gxm/include ${V}/util/include ${V}/features/include ${V}/config/include
  ${GLSLANG} ${SPIRV_CROSS})
target_compile_definitions(gxp2glsl PRIVATE BOOST_ALL_NO_LIB)
target_link_libraries(gxp2glsl PRIVATE spirv-cross-glsl fmt::fmt spdlog::spdlog Boost::filesystem SDL3::SDL3)
