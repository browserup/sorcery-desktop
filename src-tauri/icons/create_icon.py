from PIL import Image, ImageDraw

# Create a 32x32 icon for system tray (macOS template icon)
size = 32
img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Draw a simple "H" shape with a link symbol
# Left vertical bar of H
draw.rectangle([6, 8, 9, 24], fill=(0, 0, 0, 255))
# Right vertical bar of H
draw.rectangle([23, 8, 26, 24], fill=(0, 0, 0, 255))
# Horizontal bar of H
draw.rectangle([9, 15, 23, 18], fill=(0, 0, 0, 255))
# Small link chain on top right
draw.ellipse([20, 4, 24, 8], outline=(0, 0, 0, 255), width=1)
draw.ellipse([24, 4, 28, 8], outline=(0, 0, 0, 255), width=1)

img.save('icon_32x32.png')
print("Created icon_32x32.png")

# Also create a larger version for app icon
size = 512
img_large = Image.new('RGBA', (size, size), (0, 128, 255, 255))
draw_large = ImageDraw.Draw(img_large)

# Draw H shape scaled up
scale = 16
draw_large.rectangle([6*scale, 8*scale, 9*scale, 24*scale], fill=(255, 255, 255, 255))
draw_large.rectangle([23*scale, 8*scale, 26*scale, 24*scale], fill=(255, 255, 255, 255))
draw_large.rectangle([9*scale, 15*scale, 23*scale, 18*scale], fill=(255, 255, 255, 255))

img_large.save('icon_512x512.png')
print("Created icon_512x512.png")
