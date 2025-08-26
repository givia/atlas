use cgmath::prelude::*;
use winit::keyboard::KeyCode;

const EARTH_RADIUS: f64 = 6371000.0; // Radius in kilometers

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[derive(Clone, Debug)]
pub struct Camera {
    pub longitude: f64,
    pub latitude: f64,
    pub height: f64,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let theta = std::f32::consts::PI + self.longitude.to_radians() as f32;
        let phi = std::f32::consts::FRAC_PI_2 - self.latitude.to_radians() as f32;

        let distance: f32 = self.height as f32 + EARTH_RADIUS as f32;

        let eye: cgmath::Point3<f32> = cgmath::Point3::new(
            -(distance * phi.sin() * theta.cos()),
            distance * phi.cos(),
            distance * phi.sin() * theta.sin(),
        );
        let target = cgmath::Point3::new(0.0, 0.0, 0.0);
        let up = cgmath::Vector3::unit_y() * distance as f32;

        let aspect = self.screen_width as f32 / self.screen_height as f32;
        let fovy = 45.0;
        let znear = 0.1;
        let zfar = self.height as f32;

        // const PI: f64 = std::f64::consts::PI;
        // let x1 = phi - (self.height * 2.0 / (EARTH_RADIUS * 2.0 * PI)).to_radians();
        // let x2 = phi + (self.height * 2.0 / (EARTH_RADIUS * 2.0 * PI)).to_radians();
        // let y1 = theta - (self.height * 2.0 / (EARTH_RADIUS * 2.0 * PI)).to_radians();
        // let y2 = theta + (self.height * 2.0 / (EARTH_RADIUS * 2.0 * PI)).to_radians();
        // let z = ((EARTH_RADIUS + self.height)).log2();

        // // println!("x1: {}, x2: {}, y1: {}, y2: {}, z: {}", x1.to_degrees(), x2.to_degrees(), y1.to_degrees(), y2.to_degrees(), z);

        let view = cgmath::Matrix4::look_at_rh(eye, target, up);
        let proj = cgmath::perspective(cgmath::Deg(fovy), aspect, znear, zfar);

        proj * view
    }

    pub fn zoom(&mut self, zoom_level: f64) {
        self.height *= (1.05_f64).powf(-zoom_level);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = (OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();
    }
}

pub struct CameraController {
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f64, f64)>,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            mouse_pressed: false,
            last_mouse_pos: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, is_pressed: bool) -> bool {
        match key {
            KeyCode::Space => {
                self.is_up_pressed = is_pressed;
                true
            }
            KeyCode::ShiftLeft => {
                self.is_down_pressed = is_pressed;
                true
            }
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse_press(&mut self, pressed: bool) {
        self.mouse_pressed = pressed;
        if !pressed {
            self.last_mouse_pos = None;
        }
    }

    pub fn handle_mouse_wheel(&mut self, delta: f64, camera: &mut Camera) {
        camera.zoom(delta);
    }

    pub fn handle_mouse_move(&mut self, mouse_pos: (f64, f64), camera: &mut Camera) {
        if self.mouse_pressed {
            if let Some(last_pos) = self.last_mouse_pos {
                let dx = (last_pos.0 - mouse_pos.0) as f64;
                let dy = (last_pos.1 - mouse_pos.1) as f64;

                camera.longitude += dx.to_radians();
                camera.latitude -= dy.to_radians();
            }
            self.last_mouse_pos = Some(mouse_pos);
        }
    }
}
