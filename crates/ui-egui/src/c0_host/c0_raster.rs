//! A small CPU rasteriser for egui's tessellated meshes, so the grid can be drawn to pixels
//! without a window, a GPU or a presented frame.
//!
//! It plays the part of a render backend: textured triangles with per-vertex colour, premultiplied
//! alpha blended over the canvas, nearest-neighbour texture reads (glyphs are pixel-aligned at the
//! scale they are tessellated for) and the primitive's clip rectangle. Callbacks are ignored.

use std::collections::HashMap;

use egui::{
    ClippedPrimitive, ColorImage, ImageData, TextureId,
    epaint::{ImageDelta, Primitive},
};

/// An opaque RGBA canvas, premultiplied (its alpha stays 255 over an opaque start colour).
pub(super) struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, fill: [u8; 4]) -> Canvas {
        Canvas { width, height, pixels: vec![fill; width.saturating_mul(height)] }
    }

    pub fn rgba(&self) -> Vec<u8> {
        self.pixels.iter().flatten().copied().collect()
    }
}

/// Textures by id, kept up to date from a pass's `textures_delta.set`.
#[derive(Default)]
pub(super) struct Textures(HashMap<TextureId, ColorImage>);

impl Textures {
    pub fn apply<'a>(&mut self, set: impl IntoIterator<Item = (&'a TextureId, &'a [ImageDelta])>) {
        for (id, deltas) in set {
            for delta in deltas {
                let ImageData::Color(image) = &delta.image;
                match delta.pos {
                    None => {
                        self.0.insert(*id, ColorImage::clone(image));
                    }
                    Some([x0, y0]) => {
                        let Some(target) = self.0.get_mut(id) else { continue };
                        let width = target.width();
                        for row in 0..image.height() {
                            for col in 0..image.width() {
                                let Some(src) = image.pixels.get(row * image.width() + col) else { continue };
                                if x0 + col >= width {
                                    continue;
                                }
                                if let Some(dst) = target.pixels.get_mut((y0 + row) * width + x0 + col) {
                                    *dst = *src;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn draw(canvas: &mut Canvas, primitives: &[ClippedPrimitive], textures: &Textures, pixels_per_point: f32) {
    for primitive in primitives {
        let Primitive::Mesh(mesh) = &primitive.primitive else { continue };
        let Some(texture) = textures.0.get(&mesh.texture_id) else { continue };
        let clip = &primitive.clip_rect;
        let bounds = (
            (clip.min.x * pixels_per_point).floor().max(0.0) as usize,
            (clip.min.y * pixels_per_point).floor().max(0.0) as usize,
            ((clip.max.x * pixels_per_point).ceil().max(0.0) as usize).min(canvas.width),
            ((clip.max.y * pixels_per_point).ceil().max(0.0) as usize).min(canvas.height),
        );
        for &[i0, i1, i2] in mesh.indices.as_chunks::<3>().0 {
            let corner = |i: u32| mesh.vertices.get(i as usize);
            let (Some(a), Some(b), Some(c)) = (corner(i0), corner(i1), corner(i2)) else { continue };
            fill_triangle(canvas, texture, bounds, [a, b, c], pixels_per_point);
        }
    }
}

fn fill_triangle(canvas: &mut Canvas, texture: &ColorImage, (x0, y0, x1, y1): (usize, usize, usize, usize), v: [&egui::epaint::Vertex; 3], ppp: f32) {
    let p = v.map(|v| (v.pos.x * ppp, v.pos.y * ppp));
    let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
    if !area.is_finite() || area.abs() < 1e-6 {
        return;
    }
    let min_x = p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor().max(x0 as f32) as usize;
    let max_x = (p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil().max(0.0) as usize).min(x1);
    let min_y = p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor().max(y0 as f32) as usize;
    let max_y = (p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil().max(0.0) as usize).min(y1);
    let (tw, th) = (texture.width(), texture.height());
    for y in min_y..max_y {
        for x in min_x..max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let edge = |a: (f32, f32), b: (f32, f32)| ((b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0)) / area;
            let (w0, w1, w2) = (edge(p[1], p[2]), edge(p[2], p[0]), edge(p[0], p[1]));
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
            let t = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
            let tx = ((u * tw as f32) as usize).min(tw.saturating_sub(1));
            let ty = ((t * th as f32) as usize).min(th.saturating_sub(1));
            let Some(texel) = texture.pixels.get(ty * tw + tx) else { continue };
            let mut src = [0.0f32; 4];
            let texel = texel.to_array();
            for (i, out) in src.iter_mut().enumerate() {
                let vertex =
                    w0 * f32::from(v[0].color.to_array()[i]) + w1 * f32::from(v[1].color.to_array()[i]) + w2 * f32::from(v[2].color.to_array()[i]);
                *out = f32::from(texel[i]) / 255.0 * vertex / 255.0;
            }
            let Some(dst) = canvas.pixels.get_mut(y * canvas.width + x) else { continue };
            let keep = 1.0 - src[3].clamp(0.0, 1.0);
            for (d, s) in dst.iter_mut().zip(src) {
                *d = (s * 255.0 + f32::from(*d) * keep).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}
