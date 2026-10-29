use crate::framebuffer::Framebuffer;

/// Ancho y alto de cada letra. Estas dos medidas cambian según la escala
const GLYPH_WIDTH: usize = 3; // 3 píxeles de ancho
const GLYPH_HEIGHT: usize = 5; // 5 píxeles de alto 

/// Espacio en blanco entre una letra y la siguiente
const LETTER_SPACING: usize = 1; // 1 pixel de separación entre cada letra


// Dibujo de numeros en pantalla utlizando códgio binario de 3 bits:
// 1 = pixel encendido  
// 0 = pixel apagado 
fn glyph(c: char) -> [u8; GLYPH_HEIGHT] {
    match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        _ => [0b000; GLYPH_HEIGHT],
    }
}

// Cálculo del ancho que ocupa una línea de texto tomndo en cuenta la escala (N PIXÉLES EN HORIZONTAL)
pub fn text_width(text: &str, scale: usize) -> usize {
    let letters = text.chars().count();

    if letters == 0 {
        return 0;
    }

    (letters * (GLYPH_WIDTH + LETTER_SPACING) - LETTER_SPACING) * scale
}

// Calculo de la altura de una línea de texto tomando en cuenta la escala (N PIXELES EN VERTICAL)
pub fn text_height(scale: usize) -> usize {
    GLYPH_HEIGHT * scale
}

//Dibujo de un rectángulo pintado (Fondo para vvisualizar los números)
pub fn fill_rect(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    framebuffer.set_current_color(color);

    for py in y..y + height {
        for px in x..x + width {
            framebuffer.point(px, py);
        }
    }
}

/// Dibuja un texto con la esquina superior izquierda `.
pub fn draw_text(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    color: u32,
) {
    framebuffer.set_current_color(color);

    for (index, c) in text.chars().enumerate() {
        let rows = glyph(c);

        // Calculo de la separación que deben tener las letras en cada palabra del texto
        let letter_x = x + index * (GLYPH_WIDTH + LETTER_SPACING) * scale;

        for (row, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH_WIDTH {
                
                //
                let mask = 1 << (GLYPH_WIDTH - 1 - col);

                // Verificar cada dígito de cada numero binario de 3 bits (1: pintar, 0:no y repetir el ciclo)
                if bits & mask == 0 {
                    continue;
                }

                // la columna y fila desde la cual se va a comenzar a pintar
                let px = letter_x + col * scale; // Columna 
                let py = y + row * scale; // fila 

                for dy in 0..scale {
                    for dx in 0..scale {
                        framebuffer.point(px + dx, py + dy);
                    }
                }
            }
        }
    }
}

