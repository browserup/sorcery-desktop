from PIL import Image, ImageDraw

OUTPUT_DIR = 'options'

def create_s_icon():
    """Create an S letter icon similar to the E style"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)
    scale = 16

    # S shape - top curve, middle, bottom curve
    # Top horizontal bar
    draw.rectangle([9*scale, 8*scale, 24*scale, 11*scale], fill=(255, 255, 255, 255))
    # Left vertical (top half)
    draw.rectangle([6*scale, 8*scale, 9*scale, 16*scale], fill=(255, 255, 255, 255))
    # Middle horizontal bar
    draw.rectangle([9*scale, 14*scale, 23*scale, 17*scale], fill=(255, 255, 255, 255))
    # Right vertical (bottom half)
    draw.rectangle([23*scale, 16*scale, 26*scale, 24*scale], fill=(255, 255, 255, 255))
    # Bottom horizontal bar
    draw.rectangle([6*scale, 21*scale, 23*scale, 24*scale], fill=(255, 255, 255, 255))

    img.save(f'{OUTPUT_DIR}/s_letter.png')
    print(f"Created {OUTPUT_DIR}/s_letter.png")


def create_wizard_hat():
    """Create a classic wizard hat with wide brim"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)

    # Pointed cone part of hat
    cone_points = [
        (256, 40),      # tip
        (140, 340),     # bottom left of cone
        (372, 340),     # bottom right of cone
    ]
    draw.polygon(cone_points, fill=(255, 255, 255, 255))

    # Wide brim (ellipse)
    draw.ellipse([60, 310, 452, 420], fill=(255, 255, 255, 255))

    # Cut out center of brim where it meets cone (blue)
    draw.ellipse([150, 325, 362, 395], fill=(0, 128, 255, 255))

    img.save(f'{OUTPUT_DIR}/wizard_hat_brim.png')
    print(f"Created {OUTPUT_DIR}/wizard_hat_brim.png")


def create_sorcerer_hat():
    """Create a sorcerer's apprentice style hat (no brim, curved cone with stars)"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)

    # Curved cone shape (Mickey's sorcerer hat style)
    # Main cone body
    cone_points = [
        (256, 30),      # tip (slightly bent)
        (130, 440),     # bottom left
        (382, 440),     # bottom right
    ]
    draw.polygon(cone_points, fill=(255, 255, 255, 255))

    # Add a slight curve by drawing additional shape
    draw.ellipse([120, 380, 392, 480], fill=(255, 255, 255, 255))

    img.save(f'{OUTPUT_DIR}/sorcerer_hat_plain.png')
    print(f"Created {OUTPUT_DIR}/sorcerer_hat_plain.png")


def create_sorcerer_hat_with_stars():
    """Sorcerer hat with stars - dark hat, transparent stars"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)

    # Main cone body - dark (near black)
    dark_color = (30, 30, 50, 255)
    cone_points = [
        (256, 30),
        (130, 440),
        (382, 440),
    ]
    draw.polygon(cone_points, fill=dark_color)
    draw.ellipse([120, 380, 392, 480], fill=dark_color)

    # Stars are transparent (cut through to background)
    def draw_star(cx, cy, outer_r, inner_r):
        import math
        points = []
        for i in range(10):
            angle = math.pi / 2 + i * math.pi / 5
            r = outer_r if i % 2 == 0 else inner_r
            x = cx + r * math.cos(angle)
            y = cy - r * math.sin(angle)
            points.append((x, y))
        draw.polygon(points, fill=(0, 128, 255, 255))

    # Small star at top
    draw_star(256, 120, 18, 8)
    # Medium stars in middle row
    draw_star(220, 260, 24, 10)
    draw_star(292, 260, 24, 10)
    # Larger stars in bottom row
    draw_star(198, 395, 40, 18)
    draw_star(314, 395, 40, 18)

    img.save(f'{OUTPUT_DIR}/sorcerer_hat_stars.png')
    print(f"Created {OUTPUT_DIR}/sorcerer_hat_stars.png")


def create_magic_wand():
    """Simple magic wand with sparkle"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)

    # Wand (diagonal line, thick)
    wand_points = [
        (380, 420),     # bottom right (handle)
        (400, 400),
        (140, 100),     # top left (tip)
        (120, 120),
    ]
    draw.polygon(wand_points, fill=(255, 255, 255, 255))

    # Sparkle at tip
    def draw_star(cx, cy, outer_r, inner_r):
        import math
        points = []
        for i in range(8):
            angle = i * math.pi / 4
            r = outer_r if i % 2 == 0 else inner_r
            x = cx + r * math.cos(angle)
            y = cy - r * math.sin(angle)
            points.append((x, y))
        draw.polygon(points, fill=(255, 255, 255, 255))

    draw_star(110, 90, 50, 20)

    img.save(f'{OUTPUT_DIR}/magic_wand.png')
    print(f"Created {OUTPUT_DIR}/magic_wand.png")


def create_sparkle_s():
    """S with sparkle/magic effect"""
    size = 512
    img = Image.new('RGBA', (size, size), (0, 128, 255, 255))
    draw = ImageDraw.Draw(img)
    scale = 16

    # S shape
    draw.rectangle([9*scale, 8*scale, 24*scale, 11*scale], fill=(255, 255, 255, 255))
    draw.rectangle([6*scale, 8*scale, 9*scale, 16*scale], fill=(255, 255, 255, 255))
    draw.rectangle([9*scale, 14*scale, 23*scale, 17*scale], fill=(255, 255, 255, 255))
    draw.rectangle([23*scale, 16*scale, 26*scale, 24*scale], fill=(255, 255, 255, 255))
    draw.rectangle([6*scale, 21*scale, 23*scale, 24*scale], fill=(255, 255, 255, 255))

    # Add sparkle
    def draw_star(cx, cy, outer_r, inner_r):
        import math
        points = []
        for i in range(8):
            angle = i * math.pi / 4
            r = outer_r if i % 2 == 0 else inner_r
            x = cx + r * math.cos(angle)
            y = cy - r * math.sin(angle)
            points.append((x, y))
        draw.polygon(points, fill=(255, 255, 255, 255))

    draw_star(420, 90, 45, 18)

    img.save(f'{OUTPUT_DIR}/s_sparkle.png')
    print(f"Created {OUTPUT_DIR}/s_sparkle.png")


if __name__ == '__main__':
    create_s_icon()
    create_wizard_hat()
    create_sorcerer_hat()
    create_sorcerer_hat_with_stars()
    create_magic_wand()
    create_sparkle_s()
    print(f"\nAll icons created in {OUTPUT_DIR}/ folder")
