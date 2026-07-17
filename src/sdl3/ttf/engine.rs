use sdl3_ttf_sys::ttf::{TTF_GPUAtlasDrawSequence, TTF_GetGPUTextDrawData};
use sys::{gpu::SDL_GPUTexture, rect::SDL_FPoint};

use crate::{
    get_error,
    gpu::{self, Sampler, TextureSamplerBinding},
    libc::c_int,
    pixels::Color,
    render::TextureCreator,
    ttf::{
        sys::{
            TTF_CreateGPUTextEngine, TTF_CreateRendererTextEngine, TTF_CreateText,
            TTF_DestroyRendererTextEngine, TTF_DestroyText, TTF_DrawRendererText, TTF_GetTextSize,
            TTF_SetTextColor, TTF_SetTextFont, TTF_SetTextString, TTF_SetTextWrapWidth, TTF_Text,
            TTF_TextEngine, TTF_UpdateText,
        },
        Font,
    },
    Error,
};
use std::{ffi::CString, slice};

pub struct TextEngine {
    raw: *mut TTF_TextEngine,
}
impl TextEngine {
    #[doc(alias = "TTF_CreateRendererTextEngine")]
    pub fn new<T>(creator: &TextureCreator<T>) -> Result<Self, Error> {
        let raw = unsafe { TTF_CreateRendererTextEngine(creator.raw()) };
        if raw.is_null() {
            Err(get_error())
        } else {
            Ok(Self { raw })
        }
    }

    #[doc(alias = "TTF_CreateGPUTextEngine")]
    pub fn new_gpu(device: &gpu::Device) -> Result<Self, Error> {
        let raw = unsafe { TTF_CreateGPUTextEngine(device.raw()) };
        if raw.is_null() {
            Err(get_error())
        } else {
            Ok(Self { raw })
        }
    }

    pub fn raw(&self) -> *mut TTF_TextEngine {
        self.raw
    }

    #[doc(alias = "TTF_CreateText")]
    pub fn create_text(&self, font: &Font, text: &str) -> Result<Text, Error> {
        let ctext = CString::new(text).unwrap();
        let raw =
            unsafe { TTF_CreateText(self.raw, font.raw(), ctext.as_ptr(), ctext.count_bytes()) };
        if raw.is_null() {
            Err(get_error())
        } else {
            Ok(Text { raw })
        }
    }
}
impl Drop for TextEngine {
    fn drop(&mut self) {
        unsafe { TTF_DestroyRendererTextEngine(self.raw) };
    }
}

pub struct Text {
    raw: *mut TTF_Text,
}
impl Text {
    pub fn raw(&self) -> *mut TTF_Text {
        self.raw
    }

    #[doc(alias = "TTF_UpdateText")]
    pub fn update(&mut self) -> Result<(), Error> {
        let ok = unsafe { TTF_UpdateText(self.raw) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_DrawRendererText")]
    pub fn draw(&self, x: f32, y: f32) -> Result<(), Error> {
        let ok = unsafe { TTF_DrawRendererText(self.raw, x, y) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_GetTextSize")]
    pub fn size(&self) -> (u32, u32) {
        let mut w: c_int = 0;
        let mut h: c_int = 0;
        unsafe { TTF_GetTextSize(self.raw, &mut w, &mut h) };
        (w as u32, h as u32)
    }

    #[doc(alias = "TTF_SetTextFont")]
    pub fn set_font(&mut self, font: &Font) -> Result<(), Error> {
        let ok = unsafe { TTF_SetTextFont(self.raw, font.raw()) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_SetTextString")]
    pub fn set_text(&mut self, text: &str) -> Result<(), Error> {
        let ctext = CString::new(text).unwrap();
        let ok = unsafe { TTF_SetTextString(self.raw, ctext.as_ptr(), ctext.count_bytes()) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_SetTextColor")]
    pub fn set_color<T>(&mut self, color: T) -> Result<(), Error>
    where
        T: Into<Color>,
    {
        let color: Color = color.into().into();
        let ok = unsafe { TTF_SetTextColor(self.raw, color.r, color.g, color.b, color.a) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_SetTextWrapWidth")]
    pub fn set_wrap_width(&self, pixels: i32) -> Result<(), Error> {
        let ok = unsafe { TTF_SetTextWrapWidth(self.raw, pixels) };
        if ok {
            Ok(())
        } else {
            Err(get_error())
        }
    }

    #[doc(alias = "TTF_GetGPUTextDrawData")]
    pub fn get_gpu_draw_data(&self) -> Result<TextDrawData, Error> {
        let data = unsafe {
            let mut draw_data = TextDrawData::new();

            let mut sequence = TTF_GetGPUTextDrawData(self.raw);

            let mut index_offset = 0;
            let mut vertex_offset = 0;

            while !sequence.is_null() {
                Self::handle_sequence_item(
                    &mut draw_data,
                    sequence,
                    &mut index_offset,
                    &mut vertex_offset,
                );

                sequence = (*sequence).next;
            }

            draw_data
        };

        Ok(data)
    }

    unsafe fn handle_sequence_item(
        draw_data: &mut TextDrawData,
        sequence_item: *const TTF_GPUAtlasDrawSequence,
        index_offset: &mut u32,
        vertex_offset: &mut u32,
    ) {
        let sequence = sequence_item.read();

        let num_vertices = sequence.num_vertices as usize;

        let positions = slice::from_raw_parts(sequence.xy, num_vertices as usize);
        let uvs = slice::from_raw_parts(sequence.uv, num_vertices as usize);

        let mut vertices = vec![];

        for i in 0..sequence.num_vertices {
            let position = positions[i as usize];
            let uv = uvs[i as usize];

            let vertex = TextDrawVertex { position, uv };

            vertices.push(vertex);
        }

        let indices = slice::from_raw_parts(sequence.indices, sequence.num_indices as usize);

        draw_data.vertices.append(&mut vertices);
        draw_data.indices.extend_from_slice(indices);

        draw_data.stages.push(TextDrawStage {
            texture_atlas: sequence.atlas_texture,
            num_indices: sequence.num_indices as u32,
            index_offset: *index_offset,
            vertex_offset: *vertex_offset,
        });

        *index_offset += sequence.num_indices as u32;
        *vertex_offset += sequence.num_vertices as u32;
    }
}
impl Drop for Text {
    fn drop(&mut self) {
        unsafe { TTF_DestroyText(self.raw) };
    }
}

pub struct TextDrawData {
    pub vertices: Vec<TextDrawVertex>,
    pub indices: Vec<i32>,
    pub stages: Vec<TextDrawStage>,
}

impl TextDrawData {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            stages: Vec::new(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextDrawVertex {
    pub position: SDL_FPoint,
    pub uv: SDL_FPoint,
}

pub struct TextDrawStage {
    texture_atlas: *mut SDL_GPUTexture,
    num_indices: u32,
    index_offset: u32,
    vertex_offset: u32,
}

impl TextDrawStage {
    pub fn texture_sampler_binding<'a>(&self, sampler: &'a Sampler) -> TextureSamplerBinding<'a> {
        TextureSamplerBinding::new()
            .with_texture_raw(self.texture_atlas)
            .with_sampler(sampler)
    }

    pub fn num_indices(&self) -> u32 {
        self.num_indices
    }

    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    pub fn vertex_offset(&self) -> u32 {
        self.vertex_offset
    }
}
