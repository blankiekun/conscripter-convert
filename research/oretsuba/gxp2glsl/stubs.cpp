// What the recompiler needs from files left out of this build.
#include <gxm/types.h>
namespace gxm {
SceGxmTextureBaseFormat get_base_format(SceGxmTextureFormat src) {
    return static_cast<SceGxmTextureBaseFormat>(src & SCE_GXM_TEXTURE_BASE_FORMAT_MASK);
}
} // namespace gxm
namespace util::instrset {
bool hasF16C() { return false; }
}
