from PIL import Image, ImageDraw

# Create a 32x32 icon for system tray (macOS template icon)
size = 32
img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Draw a simple "E" shape
# Vertical bar of E (left side)
draw.rectangle([8, 8, 11, 24], fill=(0, 0, 0, 255))
# Top horizontal bar
draw.rectangle([11, 8, 24, 11], fill=(0, 0, 0, 255))
# Middle horizontal bar
draw.rectangle([11, 15, 21, 18], fill=(0, 0, 0, 255))
# Bottom horizontal bar
draw.rectangle([11, 21, 24, 24], fill=(0, 0, 0, 255))

img.save('icon_32x32.png')
print("Created icon_32x32.png")

# Create larger versions for app icon
for size in [128, 256, 512]:
    img_large = Image.new('RGBA', (size, size), (0, 128, 255, 255))  # Same blue color
    draw_large = ImageDraw.Draw(img_large)

    # Calculate scale factor
    scale = size / 32

    # Draw E shape scaled up
    draw_large.rectangle([8*scale, 8*scale, 11*scale, 24*scale], fill=(255, 255, 255, 255))
    draw_large.rectangle([11*scale, 8*scale, 24*scale, 11*scale], fill=(255, 255, 255, 255))
    draw_large.rectangle([11*scale, 15*scale, 21*scale, 18*scale], fill=(255, 255, 255, 255))
    draw_large.rectangle([11*scale, 21*scale, 24*scale, 24*scale], fill=(255, 255, 255, 255))

    img_large.save(f'{size}x{size}.png')
    print(f"Created {size}x{size}.png")

# Create main icon.png (512x512)
img_main = Image.new('RGBA', (512, 512), (0, 128, 255, 255))
draw_main = ImageDraw.Draw(img_main)
scale = 16

draw_main.rectangle([8*scale, 8*scale, 11*scale, 24*scale], fill=(255, 255, 255, 255))
draw_main.rectangle([11*scale, 8*scale, 24*scale, 11*scale], fill=(255, 255, 255, 255))
draw_main.rectangle([11*scale, 15*scale, 21*scale, 18*scale], fill=(255, 255, 255, 255))
draw_main.rectangle([11*scale, 21*scale, 24*scale, 24*scale], fill=(255, 255, 255, 255))

img_main.save('icon.png')
print("Created icon.png")
