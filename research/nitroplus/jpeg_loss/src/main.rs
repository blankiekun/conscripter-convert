//! The converter's JPEG copies (assets.rs make_scaled / mirrored) made
//! from a source JPEG, beside the same transform of the exe's decode.
//! Usage: jpeg_loss <src.jpg> <exe-decode.png> <zoom ZX ZY | mirror x|y|xy> <out-prefix>
//! Writes <out>.ref.png (transform of the exe decode, unencoded),
//! <out>.q90.jpg, <out>.q100.jpg, <out>.q100_444.jpg, <out>.png (the converter's pipeline).
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage};

fn transform(img: DynamicImage, args: &[String]) -> DynamicImage {
    match args[3].as_str() {
        "zoom" => {
            let (zx, zy): (i64, i64) = (args[4].parse().unwrap(), args[5].parse().unwrap());
            let (iw, ih) = (img.width() as i64, img.height() as i64);
            let (w, h) = (((iw * zx + 500) / 1000).max(1), ((ih * zy + 500) / 1000).max(1));
            img.resize_exact(w as u32, h as u32, FilterType::Triangle)
        }
        "mirror" => {
            let mut img = img;
            if args[4].contains('x') {
                img = img.fliph();
            }
            if args[4].contains('y') {
                img = img.flipv();
            }
            img
        }
        m => panic!("mode {m}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.last().unwrap();
    let copy = transform(image::open(&args[1]).unwrap(), &args).into_rgb8();
    let reference = transform(image::open(&args[2]).unwrap(), &args).into_rgb8();
    reference.save(format!("{out}.ref.png")).unwrap();
    copy.save(format!("{out}.png")).unwrap();
    for q in [90u8, 100] {
        let mut data = Vec::new();
        copy.write_with_encoder(JpegEncoder::new_with_quality(&mut std::io::Cursor::new(&mut data), q)).unwrap();
        std::fs::write(format!("{out}.q{q}.jpg"), data).unwrap();
    }
    // Quality 100 without chroma subsampling (image's encoder always subsamples).
    let enc = jpeg_encoder::Encoder::new_file(format!("{out}.q100_444.jpg"), 100).unwrap();
    let mut enc = enc;
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    enc.encode(copy.as_raw(), copy.width() as u16, copy.height() as u16, jpeg_encoder::ColorType::Rgb).unwrap();
}
