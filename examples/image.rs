use datamatrix::SymbolSize;
use image::{GrayImage, Luma};

/// Write a Data Matrix to an image
fn main() {
    // width and height of one black square
    const N: usize = 5;

    // encode "Hello, World!" using the smallest square it can fit into
    let bitmap = datamatrix::encode(b"Hello, World!", SymbolSize::MinSquare).unwrap();

    // create image with "dead space" around Data Matrix
    let width = ((bitmap.width() + 2) * N) as u32;
    let height = ((bitmap.height() + 2) * N) as u32;
    let mut image = GrayImage::from_pixel(width, height, Luma([255]));
    for (x, y) in bitmap.pixels() {
        for i in 0..N {
            for j in 0..N {
                let x_i = (x + 1) * N + j;
                let y_j = (y + 1) * N + i;
                image.put_pixel(x_i as u32, y_j as u32, Luma([0]));
            }
        }
    }
    image.save("data_matrix.png").unwrap();
}
