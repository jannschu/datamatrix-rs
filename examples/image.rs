use datamatrix::SymbolSize;
use image::{GrayImage, Luma};

/// Generate image which only contains a Data Matrix
fn main() {
    // Define width and height in pixels of one black square in the image.
    // Be careful if your space constraints result in non integer sizes for
    // a black square. In this case you might want to generate smaller image
    // and then interpolate (rescale).
    const N: usize = 5;

    // Encode "Hello, World!" using the smallest square it can fit into
    let bitmap = datamatrix::encode(b"Hello, World!", SymbolSize::MinSquare).unwrap();

    // Create an image with "dead space" which only contains the Data Matrix
    let width = ((bitmap.width() + 2) * N) as u32;
    let height = ((bitmap.height() + 2) * N) as u32;
    let mut image = GrayImage::from_pixel(width, height, Luma([255]));
    for (x, y) in bitmap.pixels() {
        // Write the black square at x, y using NxN black pixels
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
