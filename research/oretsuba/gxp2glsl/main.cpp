// Prints the GLSL Vita3K's recompiler makes of a GXP shader (see CMakeLists.txt).
#include <shader/spirv_recompiler.h>
#include <util/log.h>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <iterator>

int main(int argc, char **argv) {
    std::ifstream f(argv[1], std::ios::binary);
    std::vector<char> data((std::istreambuf_iterator<char>(f)), {});
    FeatureState features{ .support_shader_interlock = true, .direct_fragcolor = false };
    shader::Hints hints{ .attributes = nullptr, .color_format = SCE_GXM_COLOR_FORMAT_U8U8U8U8_ABGR };
    std::fill_n(hints.vertex_textures, SCE_GXM_MAX_TEXTURE_UNITS, SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR);
    std::fill_n(hints.fragment_textures, SCE_GXM_MAX_TEXTURE_UNITS, SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR);
    auto out = shader::convert_gxp(*reinterpret_cast<SceGxmProgram *>(data.data()), "x", features, shader::Target::GLSLOpenGL, hints, false, true,
        [](const std::string &ext, const std::string &dump) { if (ext == "dsm") std::cerr << dump; return true; });
    std::cout << out.glsl << std::endl;
}
