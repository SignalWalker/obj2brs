use cgmath::Vector4;

pub fn modulus(a: f32, b: f32) -> f32 {
    ((a % b) + b) % b
}

pub fn float_equals(a: f32, b: f32) -> bool {
    let error_margin = std::f32::EPSILON;
    (b - a).abs() < error_margin
}

pub fn rgb2hsv(rgb: Vector4::<u8>) -> Vector4::<f32> {
	let r = (rgb[0] as f32)/255f32;
	let g = (rgb[1] as f32)/255f32;
    let b = (rgb[2] as f32)/255f32;
    let a = (rgb[3] as f32)/255f32;

    // max of rgb is equivalent to V in HSV
    let mut max = r;
    if g > max {max = g}
    if b > max {max = b}

    // min of rgb is V - C where C is chroma (range)
    let mut min = r;
    if g < min {min = g}
    if b < min {min = b}

    let mid = max - min;
    
    let mut hue = if float_equals(mid, 0f32) {
		0f32
	} else if float_equals(max, r) {
        modulus((g - b) / mid, 6f32)
	} else if float_equals(max, g) {
		(b - r) / mid + 2f32
	} else if float_equals(max, b) {
		(r - g) / mid + 4f32
	} else {
        0f32
    };

    hue *= std::f32::consts::PI / 3f32;
    if hue < 0f32 { hue += 2f32 * std::f32::consts::PI }

    let saturation = if float_equals(max, 0f32) {
		0f32
	} else {
		mid / max
	};

    Vector4::<f32>::new(hue, saturation, max, a)
}

pub fn hsv2rgb(hsv: Vector4::<f32>) -> Vector4::<u8> {
    let hue = hsv[0] * 180f32 / std::f32::consts::PI;
    let saturation = hsv[1];
    let value = hsv[2];

    let chroma = value * saturation;
    let x = chroma * (1f32 - (modulus(hue/60f32, 2f32) - 1f32).abs());
    let match_value = value - chroma;

    let (r, g, b) = match hue {
        hh if hh < 60f32  => (chroma, x, 0f32),
        hh if hh < 120f32 => (x, chroma, 0f32),
        hh if hh < 180f32 => (0f32, chroma, x),
        hh if hh < 240f32 => (0f32, x, chroma),
        hh if hh < 300f32 => (x, 0f32, chroma),
        hh if hh < 360f32 => (chroma, 0f32, x),
        _ => (0f32, 0f32, 0f32)
    };

    Vector4::new(((r + match_value)*255f32) as u8, ((g + match_value)*255f32) as u8, ((b + match_value)*255f32) as u8, (hsv[3]*255f32) as u8)
}

pub fn hsv_distance(a: &Vector4::<f32>, b: &Vector4::<f32>) -> f32 {
    // (a.x - b.x).powf(2.0) * 8./21.
    // + (a.y - b.y).powf(2.0) * 5./21.
    // + (a.z - b.z).powf(2.0) * 4./21.
    // + (a.w - b.w).powf(2.0) * 4./21.

    (a.x.sin()*a.y - b.x.sin()*b.y).powf(2.0)
    + (a.x.cos()*a.y - b.x.cos()*b.y).powf(2.0)
    + (a.z - b.z).powf(2.0)
    + (a.w - b.w).powf(2.0)
}

pub fn hsv_average(colors: &[Vector4::<u8>]) -> Vector4::<f32> {
    let n = colors.len() as f32;
    let mut h_avg = 0f32;
    let mut s_avg = 0f32;
    let mut v_avg = 0f32;
    let mut a_avg = 0f32;

    for c in colors {
        let color = rgb2hsv(*c);
        h_avg += color.x;
        s_avg += color.y;
        v_avg += color.z;
        a_avg += color.w;
    }

    Vector4::<f32>::new(h_avg/n, s_avg/n, v_avg/n, a_avg/n)
}

pub fn convert_colorset_to_hsv(colorset: &[brs::Color]) -> Vec::<Vector4::<f32>> {
    let mut new = Vec::<Vector4::<f32>>::with_capacity(colorset.len());
    for c in colorset {
        new.push(rgb2hsv(Vector4::new(c.r(), c.g(), c.b(), c.a())));
    }

    new
}

pub fn match_hsv_to_colorset(colorset: &[Vector4::<f32>], color: &Vector4::<f32>) -> usize {
    let mut min = 0;
    let mut min_distance = hsv_distance(&colorset[0], &color);
    for (i, cs) in colorset.iter().enumerate() {
        let distance = hsv_distance(&cs, &color);
        if distance < min_distance {
            min_distance = distance;
            min = i;
        }
    }

    min
}