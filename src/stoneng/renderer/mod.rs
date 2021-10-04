#![allow(dead_code)]
use crate::EngineError;
use crate::shader;
use crate::ecs::component;

use stb::image::LoadResult;
use std::{
    path::Path,
    mem::size_of,
};
use glm::{Vec2, Vec3, Vec4};
use gl::types::*;

#[repr(C)]
#[derive(Debug)]
pub struct RenderSprite {
    pub translation:    (f32, f32, f32),
    pub scale:          (f32, f32),
    pub rotation:        f32,
    pub color:          (f32, f32, f32, f32),
    pub sprite_id:      u16,
    pub sprite_dims:    u8,
    pub sprite_flags:   u8,
}
impl Default for RenderSprite {
    fn default() -> Self {
        Self {
            translation: (0.0, 0.0, 0.0),
            scale:       (1.0, 1.0),
            rotation:    0.0,
            
            color:       (1.0, 1.0, 1.0, 1.0),
            
            sprite_id:    0,
            sprite_dims:  0,
            sprite_flags: 0,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct SpriteRenderer {
    initialized: bool,

    shader:     GLuint,
    vao:        GLuint,
    abo:        GLuint,
    tex:        GLuint,
    uniform_locations:   [GLint; 3],
}

impl SpriteRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, atlas: &[u8]) -> Result<(), EngineError> {
        if self.initialized { return Ok(()) }

        // Prevent running this function too early.
        if !gl::Viewport::is_loaded() {
            let msg = format!("{}\n{}",
                "SpriteRenderer::init called before gl bindings were loaded.",
                "init() should only be called by the engine."
            );
            return Err(EngineError::RendererInit(msg));
        }

        // Load the image file into memory and format using stb_image
        let atlas_img = match stb::image::load_from_memory(atlas){
            // Accept unsigned byte formatted image
            LoadResult::ImageU8(img) => img,
            // Error on any other result
            _ => {
                let msg = format!("{}\n{}",
                        "Failed to load texture atlas.",
                        "Ensure the atlas is a RGBA PNG."
                    );  
                return Err(EngineError::RendererInit(msg));
            },
        };
 
        // Build shader programs
        self.shader = shader::program_from_sources(
            include_str!("../../../assets/shaders/sprite/vert.glsl").into(),
            include_str!("../../../assets/shaders/sprite/frag.glsl").into(),
            Some(include_str!("../../../assets/shaders/sprite/geom.glsl").into())
        ).unwrap();
 
        unsafe {
            // Generate OpenGL objects/buffers
            gl::GenVertexArrays(1, &mut self.vao as *mut GLuint);
            gl::GenBuffers(1, &mut self.abo as *mut GLuint);
            gl::GenTextures(1, &mut self.tex as *mut GLuint);
            
            // Binding
            gl::BindVertexArray(self.vao);
            gl::BindTexture(gl::TEXTURE_2D, self.tex);

            // Load the texture to the GPU
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGBA as i32, 
                atlas_img.width as i32, atlas_img.height as i32, 0,
                gl::RGBA, gl::UNSIGNED_BYTE,
                atlas_img.data.as_ptr() as *const GLvoid
            );
            

            // Set up the attribute pointers
            let stride = size_of::<RenderSprite>() as i32;
            // Transform
                // Translation
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abo);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, 0 as *const GLvoid); 

                // Scale
            let scale_offset = size_of::<f32>() as i32 * 3;
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, 
                                    scale_offset as *const GLvoid); 
                // Rotation    
            let rotation_offset = scale_offset + (size_of::<f32>() as i32) * 2;
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, 
                                    rotation_offset as *const GLvoid); 
            
            // Color 
            let color_offset = rotation_offset + (size_of::<f32>() as i32);
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, stride, 
                                    color_offset as *const GLvoid);  

            // Sprite Data
            let data_offset = color_offset + (size_of::<f32>() as i32) * 4;
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribIPointer(4, 1, gl::UNSIGNED_INT, stride, 
                                     data_offset as *const GLvoid); 

            // Find and store the uniform locations
            self.uniform_locations[0] = shader::get_uniform_location(
                self.shader, "view_projection"); 
            self.uniform_locations[1] = shader::get_uniform_location(
                self.shader, "sheet_width");                       
            self.uniform_locations[2] = shader::get_uniform_location(
                self.shader, "sheet_tile_w");           

            // Unbinding
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::UseProgram(0);
        }

        self.initialized = true;
        Ok(())
    }

    pub fn render(&self, sprites: &Vec<RenderSprite>){

        if !self.initialized { return; }
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            
            gl::UseProgram(self.shader);
            gl::BindVertexArray(self.vao);
            gl::BindTexture(gl::TEXTURE_2D, self.tex);
            
            // Set uniforms
            let view_projection = glm::ortho(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);
            gl::UniformMatrix4fv(self.uniform_locations[0], 1, gl::FALSE, 
                                 view_projection.as_ptr());

            gl::Uniform1i(self.uniform_locations[1], 256);
            gl::Uniform1i(self.uniform_locations[2], 32);

            // Transfer sprite data
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abo);
            gl::BufferData(
                gl::ARRAY_BUFFER, 
                (size_of::<RenderSprite>() * sprites.len()) as GLsizeiptr,
                sprites.as_ptr() as *const GLvoid,
                gl::DYNAMIC_DRAW
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);

            // Render to main framebuffer
            gl::DrawArrays(gl::POINTS, 0, sprites.len() as i32);
         
            // Unbind
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
 
    }
}

#[derive(Default)]
struct LightRenderer {
    initialized:    bool,
    dithered:       bool,
    dither_scale:   f32,
    
    fbo:        GLuint,
    shader:     [GLuint; 2],
    vaos:       [GLuint; 2],
    ebo:        GLuint,
    abos:       [GLuint; 3],
    tex:        GLuint,
    uniform_locations:   [GLint; 3],
}
impl LightRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) -> Result<(), EngineError> {
         if self.initialized { return Ok(()) }

        // Prevent running this function too early.
        if !gl::Viewport::is_loaded() {
            let msg = format!("{}\n{}",
                "LightRenderer::init called before gl bindings were loaded.",
                "init() should only be called by the engine."
            );
            return Err(EngineError::RendererInit(msg));
        }
 
        // Build shader programs
        self.shader[0] = shader::program_from_sources(
            include_str!("../../../assets/shaders/lightmap/vert.glsl").into(),
            include_str!("../../../assets/shaders/lightmap/frag.glsl").into(),
            Some(include_str!("../../../assets/shaders/lightmap/geom.glsl").into())
        ).unwrap();
        self.shader[1] = shader::program_from_sources(
            include_str!("../../../assets/shaders/shadowmask/vert.glsl").into(),
            include_str!("../../../assets/shaders/shadowmask/frag.glsl").into(),
            None,
        ).unwrap();  
        
        unsafe {
            // Generate OpenGL objects/buffers
            gl::GenFramebuffers(1, &mut self.fbo as *mut GLuint);
            gl::GenVertexArrays(2, &mut self.vaos as *mut GLuint);
            gl::GenBuffers(3, &mut self.abos as *mut GLuint);
            gl::GenTextures(1, &mut self.tex as *mut GLuint);
            
            // ===================== Lightmap =======================
            gl::BindVertexArray(self.vaos[0]);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[0]);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo); 
            gl::BindTexture(gl::TEXTURE_2D, self.tex);

            // Load the texture to the GPU
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGB as i32, 
                1920, 1080, 0, 
                gl::RGB, gl::UNSIGNED_BYTE, 
                std::ptr::null()
            );
            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, self.tex, 0); 
         
        
            let stride = size_of::<Vec3>() as i32;
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, 0 as *const GLvoid);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[1]);

            let intensity_offset = (size_of::<f32>() * 2) as i32;
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 1, gl::FLOAT, gl::FALSE, 
                                    stride, intensity_offset as *const GLvoid); 
            // Unbinding
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::UseProgram(0);
        }

        self.initialized = true;
        Ok(())

    }
}


/*
        unsafe {

        
            // ============= LIGHTMAP RENDERER ===============
            gl::GenFramebuffers(1, &mut self.fbos as *mut GLuint);

            // == Setup Lightmap buffer ==
            gl::UseProgram(self.programs[1]); 
            gl::BindVertexArray(self.vaos[1]);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[1]);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbos); 
            gl::BindTexture(gl::TEXTURE_2D, self.textures[1]);
            
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            // Empty texture
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGB as i32, 
                800, 600, 0, 
                gl::RGB, gl::UNSIGNED_BYTE, 
                std::ptr::null()
            );
            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, self.textures[1], 0); 

            // == Lightmap attribute pointers == 
            stride = size_of::<Vec3>() as i32;
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, 0 as *const GLvoid);
            
            let intensity_offset = (size_of::<f32>() * 2) as i32;
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 1, gl::FLOAT, gl::FALSE, 
                                    stride, intensity_offset as *const GLvoid);
            
            // Unbind objects
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            // ============================================
           
            // ================= SHADOWMASK RENDERER ==================
            gl::GenBuffers(1, &mut self.ebos as *mut GLuint);
            
            gl::UseProgram(self.programs[2]);
            
            gl::BindVertexArray(self.vaos[2]);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[2]);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebos);
            
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 16, 0 as *const GLvoid);
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 16, 8 as *const GLvoid);

            let screen_quad: [f32; 16] = [
                 1.0,  1.0, 1.0, 1.0,
                 1.0, -1.0, 1.0, 0.0,
                -1.0, -1.0, 0.0, 0.0,
                -1.0,  1.0, 0.0, 1.0,
            ];
            let scr_quad_ind: [GLuint; 6] = [
                0, 1, 3,
                1, 2, 3,
            ];

            gl::BufferData(
                gl::ARRAY_BUFFER,  
                (size_of::<f32>() * screen_quad.len()) as GLsizeiptr,
                screen_quad.as_ptr() as *const GLvoid,
                gl::STATIC_DRAW
            );
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (size_of::<GLuint>() * scr_quad_ind.len()) as GLsizeiptr,
                scr_quad_ind.as_ptr() as *const GLvoid,
                gl::STATIC_DRAW
            );

            // ==========================================================

            // Unbind all
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::UseProgram(0); 

        }


        self.initialized = true; 
        Ok(())
    }

    pub fn render(&mut self) {
        if !self.initialized { return; }
        let res = Vec2::from([800.0, 600.0]);
        let dither_scale: f32 = self.dither_scale;

        unsafe {
            // ===== Render Sprites ======
            // Takes a set of sprite data, maps them to an atlas
            // and draws them on screen.
            // ---------------------------
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::Enable(gl::DEPTH_TEST);

            gl::UseProgram(self.programs[0]);
            gl::BindVertexArray(self.vaos[0]);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[0]);
            gl::BindTexture(gl::TEXTURE_2D, self.textures[0]);
            
            // Set uniforms
            let view_projection = glm::ortho(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);
            gl::UniformMatrix4fv(self.sprite_ulocs[0], 1, gl::FALSE, view_projection.as_ptr());
            gl::Uniform1i(self.sprite_ulocs[1], 256);
            gl::Uniform1i(self.sprite_ulocs[2], 32);

            // Transfer sprite data
            gl::BufferData(
                gl::ARRAY_BUFFER, 
                (size_of::<Sprite>() * self.sprites.len()) as GLsizeiptr,
                self.sprites.as_ptr() as *const GLvoid,
                gl::DYNAMIC_DRAW
            );

            // Render to main buffer
            gl::DrawArrays(gl::POINTS, 0, self.sprites.len() as i32);
         
            // Unbind
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            // ===============================


            // ======= Render lightmap ========
            // Takes an array of 2d points with intensity values and generates
            // a set of circles using shaders. These are rendered to a texture
            // to be used as a lightmap.
            // --------------------------------
            gl::Disable(gl::DEPTH_TEST);
            
            gl::UseProgram(self.programs[1]);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbos);
            gl::BindTexture(gl::TEXTURE_2D, self.textures[1]);
            gl::BindVertexArray(self.vaos[1]);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[1]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BlendFunc(gl::ONE, gl::ONE);
            //gl::BlendEquation(gl::MAX); 
            
            let down_res = res / dither_scale;
            let scaled_vp = glm::ortho(0.0, down_res.x, 0.0, down_res.y, -1.0, 1.0); 
            gl::Viewport(0, 0, down_res.x as i32, down_res.y as i32); 
            
            gl::UniformMatrix4fv(
                shader::get_uniform_location(self.programs[1], "view_projection"),
                1, gl::FALSE, scaled_vp.as_ptr()
            );
            gl::Uniform1f(
                shader::get_uniform_location(self.programs[1], "px_scale"),
                dither_scale,
            );

            // Load point light data
            gl::BufferData(
                gl::ARRAY_BUFFER, 
                (size_of::<Vec3>() * self.lights.len()) as GLsizeiptr,
                self.lights.as_ptr() as *const GLvoid,
                gl::DYNAMIC_DRAW
            );

            // Render lightmap to texture
            gl::DrawArrays(gl::POINTS, 0, self.lights.len() as i32);
            
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::BlendEquation(gl::FUNC_ADD); 
            
            // Unbind
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0); 
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::Viewport(0, 0, 800, 600); 
            // ===============================
            

            // ===== Render shadow mask ====== 
            // Renders a quad across the whole screen, using the lightmap
            // the mask is discarded in some areas using bayesian dithering, simulating light.
            // -------------------------------
            gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ONE); 
            gl::UseProgram(self.programs[2]);
            gl::BindVertexArray(self.vaos[2]);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, self.abos[2]);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebos);
            
            gl::BindTexture(gl::TEXTURE_2D, self.textures[1]);
           
            gl::Uniform1f(
                shader::get_uniform_location(self.programs[2], "lightmap_scale"),
                dither_scale,
            );

            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const GLvoid);
            // ===============================
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::UseProgram(0);
        }

        // Order sprites so that they are sent front to back
        //self.sprites.sort_by(|a,b|  b.partial_cmp(a).unwrap() );
    }
} */
