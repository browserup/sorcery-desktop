from PIL import Image, ImageDraw
import math

STAR_CENTER_X = 370
STAR_CENTER_Y = 145

def lerp_color(c1, c2, t):
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))

def get_gradient_color(distance, max_distance):
    colors = [
        (0.0, (147, 51, 234)),    # #9333ea - purple
        (0.7, (192, 38, 211)),    # #c026d3 - magenta
        (1.0, (245, 158, 11)),    # #f59e0b - amber/orange
    ]

    t = min(distance / max_distance, 1.0) if max_distance > 0 else 0

    for i in range(len(colors) - 1):
        if t <= colors[i + 1][0]:
            t_local = (t - colors[i][0]) / (colors[i + 1][0] - colors[i][0])
            return lerp_color(colors[i][1], colors[i + 1][1], t_local)

    return colors[-1][1]

def draw_four_point_star_gradient(img, cx, cy, outer_r, inner_r):
    points = []
    for i in range(8):
        angle = -math.pi / 2 + i * math.pi / 4
        r = outer_r if i % 2 == 0 else inner_r
        x = cx + r * math.cos(angle)
        y = cy + r * math.sin(angle)
        points.append((x, y))

    mask = Image.new('L', img.size, 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.polygon(points, fill=255)

    min_x = int(cx - outer_r)
    max_x = int(cx + outer_r) + 1
    min_y = int(cy - outer_r)
    max_y = int(cy + outer_r) + 1

    pixels = img.load()
    mask_pixels = mask.load()
    max_dist = outer_r

    for y in range(max(0, min_y), min(img.size[1], max_y)):
        for x in range(max(0, min_x), min(img.size[0], max_x)):
            if mask_pixels[x, y] > 0:
                dist = math.sqrt((x - cx) ** 2 + (y - cy) ** 2)
                color = get_gradient_color(dist, max_dist)
                pixels[x, y] = color + (255,)

def draw_white_mini_star(img, cx, cy, outer_r, inner_r):
    draw = ImageDraw.Draw(img)
    points = []
    for i in range(8):
        angle = -math.pi / 2 + i * math.pi / 4
        r = outer_r if i % 2 == 0 else inner_r
        x = cx + r * math.cos(angle)
        y = cy + r * math.sin(angle)
        points.append((x, y))
    draw.polygon(points, fill=(255, 255, 255, 255))

def draw_wand(draw, base_scale, color):
    wand_points = [
        (52 * base_scale, 428 * base_scale),
        (87 * base_scale, 463 * base_scale),
        (328 * base_scale, 230 * base_scale),
        (293 * base_scale, 195 * base_scale),
    ]
    draw.polygon(wand_points, fill=color)

def create_wand_icon(output_path='wand_icon_new.png', size=512):
    scale = 2
    canvas_size = size * scale
    base_scale = canvas_size / 512

    img = Image.new('RGBA', (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    draw_wand(draw, base_scale, (0, 0, 0, 255))

    cx = STAR_CENTER_X * base_scale
    cy = STAR_CENTER_Y * base_scale

    draw_four_point_star_gradient(
        img, cx, cy,
        outer_r=115 * base_scale,
        inner_r=40 * base_scale
    )

    draw_white_mini_star(
        img, cx, cy,
        outer_r=20 * base_scale,
        inner_r=7 * base_scale
    )

    img = img.resize((size, size), Image.LANCZOS)

    img.save(output_path)
    print(f"Created {output_path} ({size}x{size})")
    return img

def draw_yellow_glow(img, cx, cy, glow_radius):
    """Draw a yellow radial glow centered at (cx, cy)"""
    pixels = img.load()

    min_x = max(0, int(cx - glow_radius))
    max_x = min(img.size[0], int(cx + glow_radius) + 1)
    min_y = max(0, int(cy - glow_radius))
    max_y = min(img.size[1], int(cy + glow_radius) + 1)

    for y in range(min_y, max_y):
        for x in range(min_x, max_x):
            dist = math.sqrt((x - cx) ** 2 + (y - cy) ** 2)
            if dist <= glow_radius:
                # Fade alpha from center (more opaque) to edge (transparent)
                t = dist / glow_radius
                alpha = int(255 * (1 - t * t))  # Quadratic falloff, full opacity at center
                if alpha > 0:
                    # Yellow color: (250, 204, 21)
                    existing = pixels[x, y]
                    if existing[3] == 0:  # Transparent pixel
                        pixels[x, y] = (250, 204, 21, alpha)
                    else:
                        # Blend with existing pixel
                        blend_alpha = alpha / 255
                        new_r = int(existing[0] * (1 - blend_alpha) + 250 * blend_alpha)
                        new_g = int(existing[1] * (1 - blend_alpha) + 204 * blend_alpha)
                        new_b = int(existing[2] * (1 - blend_alpha) + 21 * blend_alpha)
                        new_a = max(existing[3], alpha)
                        pixels[x, y] = (new_r, new_g, new_b, new_a)


def create_active_icon(output_path='wand_icon_active.png', size=512):
    """Create icon with yellow glow behind the star for 'casting' state"""
    scale = 2
    canvas_size = size * scale
    base_scale = canvas_size / 512

    img = Image.new('RGBA', (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    cx = STAR_CENTER_X * base_scale
    cy = STAR_CENTER_Y * base_scale

    # Draw yellow glow FIRST (behind everything)
    # Large radius to be visible at small tray icon sizes
    glow_radius = 280 * base_scale
    draw_yellow_glow(img, cx, cy, glow_radius)

    # Then draw wand on top
    draw_wand(draw, base_scale, (0, 0, 0, 255))

    # Then draw the star gradient
    draw_four_point_star_gradient(
        img, cx, cy,
        outer_r=115 * base_scale,
        inner_r=40 * base_scale
    )

    # Finally the white mini star
    draw_white_mini_star(
        img, cx, cy,
        outer_r=20 * base_scale,
        inner_r=7 * base_scale
    )

    img = img.resize((size, size), Image.LANCZOS)
    img.save(output_path)
    print(f"Created {output_path} ({size}x{size})")
    return img


def generate_all_icons():
    """Generate all required icon sizes for Tauri app"""
    sizes = [32, 128, 256, 512]

    for size in sizes:
        create_wand_icon(f'{size}x{size}.png', size)

    # Main icon.png (512x512)
    create_wand_icon('icon.png', 512)

    # Active state icons for tray animation
    for size in [32, 128, 256, 512]:
        create_active_icon(f'{size}x{size}_active.png', size)
    create_active_icon('icon_active.png', 512)

    print("\nAll icons generated!")

if __name__ == '__main__':
    generate_all_icons()
