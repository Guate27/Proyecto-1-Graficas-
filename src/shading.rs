// Distancia en la que una pared en la que se ve con su brillo completo 
const FOG_START: f32 = 100.0; // PARED A 100 PIXELES DEL JUGADOR => PARED QUE ESTA CERCA DEL JUGADOR: SE LOGRA VER PERFECTAMENTE

// Distancia en la que una pared llegan a su brillo mínimo. 
const FOG_END: f32 = 1200.0; // PARED A 1200 PIXELES DEL JUGADOR => PARED QUE ESTA LEJOS DEL JUGADOR: SE VE OSCURA

//  BRILLO MÍNIMO: 
const MIN_BRIGHTNESS: f32 = 0.15; //15% DEL BRILLO

// Las paredes se pintan un poco más oscuras que el techo y el suelo. 
pub const HORIZONTAL_FACE_BRIGHTNESS: f32 = 0.75;

// Separa cada par de dígitos del código hexadecimal de un color del modelo RGB 
fn split(color: u32) -> (f32, f32, f32) {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;

    (r, g, b)
}

// Vuelve a juntar los tres pares de dígitos del código hexadecimal para volver a formar un solo color del modelo RGB. 
fn join(r: f32, g: f32, b: f32) -> u32 {
    let r = r.clamp(0.0, 255.0) as u32;
    let g = g.clamp(0.0, 255.0) as u32;
    let b = b.clamp(0.0, 255.0) as u32;

    (r << 16) | (g << 8) | b
}

// Aplica un nivel determinado de brillo a un color
pub fn shade(color: u32, brightness: f32) -> u32 {
    let (r, g, b) = split(color);

    join(r * brightness, g * brightness, b * brightness)
}

// Mezcla dos colores. Sirve para hacer degradados.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let (ar, ag, ab) = split(a);
    let (br, bg, bb) = split(b);

    join(
        ar + (br - ar) * t,
        ag + (bg - ag) * t,
        ab + (bb - ab) * t,
    )
}


// Calcula el nivel de brillo del color de algo segun su distancia respecto al jugador
pub fn distance_brightness(distance: f32) -> f32 {
    let fog = (distance - FOG_START) / (FOG_END - FOG_START);
    let fog = fog.clamp(0.0, 1.0);

    1.0 - fog * (1.0 - MIN_BRIGHTNESS)
}