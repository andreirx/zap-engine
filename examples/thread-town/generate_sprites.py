#!/usr/bin/env python3
"""Generate placeholder sprites for Thread Town example."""

from PIL import Image, ImageDraw, ImageFont
import os

# Output directory
out_dir = os.path.join(os.path.dirname(__file__), "public", "assets")
os.makedirs(out_dir, exist_ok=True)

# Colors
BLUE = (52, 152, 219, 255)       # Robot A
ORANGE = (230, 126, 34, 255)     # Robot B
DARK_BG = (30, 30, 50, 255)      # Background
GRAY = (100, 100, 120, 255)      # Counter box
GOLD = (241, 196, 15, 255)       # Key
WHITE = (255, 255, 255, 255)
RED = (231, 76, 60, 255)
GREEN = (46, 204, 113, 255)
TRANSPARENT = (0, 0, 0, 0)

# Cell size for main atlas (4x4 = 512x512 total)
CELL_SIZE = 128

def draw_robot(draw, x, y, color, size=CELL_SIZE):
    """Draw a simple robot icon."""
    margin = size // 8
    # Body
    body_rect = [x + margin*2, y + size//3, x + size - margin*2, y + size - margin]
    draw.rounded_rectangle(body_rect, radius=8, fill=color)
    # Head
    head_rect = [x + size//3, y + margin, x + size - size//3, y + size//3 + margin]
    draw.rounded_rectangle(head_rect, radius=6, fill=color)
    # Eyes
    eye_y = y + size//5
    draw.ellipse([x + size//3 + 8, eye_y, x + size//3 + 20, eye_y + 12], fill=WHITE)
    draw.ellipse([x + size - size//3 - 20, eye_y, x + size - size//3 - 8, eye_y + 12], fill=WHITE)
    # Antenna
    draw.rectangle([x + size//2 - 2, y + 4, x + size//2 + 2, y + margin], fill=color)
    draw.ellipse([x + size//2 - 6, y, x + size//2 + 6, y + 12], fill=color)

def draw_counter_box(draw, x, y, size=CELL_SIZE):
    """Draw a counter/memory box."""
    margin = size // 8
    # Outer box
    draw.rounded_rectangle([x + margin, y + margin, x + size - margin, y + size - margin],
                          radius=10, fill=GRAY, outline=WHITE, width=3)
    # Label
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 20)
    except:
        font = ImageFont.load_default()
    draw.text((x + size//2, y + size//2), "MEM", fill=WHITE, font=font, anchor="mm")

def draw_key(draw, x, y, size=CELL_SIZE):
    """Draw a key (mutex) icon."""
    cx, cy = x + size//2, y + size//2
    # Key head (circle)
    draw.ellipse([cx - 25, cy - 35, cx + 25, cy + 5], fill=GOLD)
    draw.ellipse([cx - 12, cy - 22, cx + 12, cy - 2], fill=TRANSPARENT)
    # Key shaft
    draw.rectangle([cx - 6, cy, cx + 6, cy + 30], fill=GOLD)
    # Key teeth
    draw.rectangle([cx + 6, cy + 15, cx + 15, cy + 22], fill=GOLD)
    draw.rectangle([cx + 6, cy + 25, cx + 12, cy + 30], fill=GOLD)

def draw_thought_bubble(draw, x, y, size=CELL_SIZE):
    """Draw a thought bubble."""
    cx, cy = x + size//2, y + size//3
    # Main bubble
    draw.ellipse([cx - 40, cy - 30, cx + 40, cy + 30], fill=WHITE)
    # Small circles below
    draw.ellipse([cx - 20, cy + 25, cx - 5, cy + 40], fill=WHITE)
    draw.ellipse([cx - 30, cy + 38, cx - 20, cy + 48], fill=WHITE)

def draw_bell(draw, x, y, size=CELL_SIZE):
    """Draw a bell (notify) icon."""
    cx, cy = x + size//2, y + size//2
    # Bell body
    draw.polygon([(cx - 30, cy + 10), (cx, cy - 40), (cx + 30, cy + 10)], fill=GOLD)
    draw.ellipse([cx - 35, cy, cx + 35, cy + 25], fill=GOLD)
    # Clapper
    draw.ellipse([cx - 8, cy + 18, cx + 8, cy + 34], fill=(180, 150, 10, 255))

def draw_lock(draw, x, y, size=CELL_SIZE, closed=True):
    """Draw a lock icon."""
    cx, cy = x + size//2, y + size//2
    # Lock body
    draw.rounded_rectangle([cx - 25, cy - 5, cx + 25, cy + 35], radius=5, fill=GRAY)
    # Shackle
    if closed:
        draw.arc([cx - 18, cy - 35, cx + 18, cy + 5], 0, 180, fill=GRAY, width=8)
    else:
        draw.arc([cx - 18, cy - 45, cx + 18, cy - 5], 45, 180, fill=GRAY, width=8)
    # Keyhole
    draw.ellipse([cx - 6, cy + 5, cx + 6, cy + 17], fill=DARK_BG)
    draw.polygon([(cx - 4, cy + 15), (cx + 4, cy + 15), (cx, cy + 28)], fill=DARK_BG)

def draw_zzz(draw, x, y, size=CELL_SIZE):
    """Draw a sleep ZZZ icon."""
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 32)
    except:
        font = ImageFont.load_default()
    draw.text((x + size//2, y + size//2 - 15), "Z", fill=WHITE, font=font, anchor="mm")
    draw.text((x + size//2 + 15, y + size//2), "z", fill=WHITE, font=font, anchor="mm")
    draw.text((x + size//2 + 25, y + size//2 + 15), "z", fill=(200, 200, 200, 255), font=font, anchor="mm")

def draw_exclamation(draw, x, y, size=CELL_SIZE):
    """Draw an exclamation mark."""
    cx, cy = x + size//2, y + size//2
    draw.rounded_rectangle([cx - 8, cy - 35, cx + 8, cy + 10], radius=4, fill=RED)
    draw.ellipse([cx - 8, cy + 18, cx + 8, cy + 34], fill=RED)

def draw_checkmark(draw, x, y, size=CELL_SIZE):
    """Draw a checkmark."""
    cx, cy = x + size//2, y + size//2
    draw.line([(cx - 25, cy), (cx - 5, cy + 20), (cx + 30, cy - 25)], fill=GREEN, width=10)

# ── Generate main sprites atlas (4x4 = 512x512) ────────────────────────
print("Generating sprites_4x4.png...")
img = Image.new("RGBA", (CELL_SIZE * 4, CELL_SIZE * 4), TRANSPARENT)
draw = ImageDraw.Draw(img)

# Row 0: background, robot_blue, robot_orange, counter_box
draw.rectangle([0, 0, CELL_SIZE, CELL_SIZE], fill=DARK_BG)  # background
draw_robot(draw, CELL_SIZE, 0, BLUE)                         # robot_blue
draw_robot(draw, CELL_SIZE * 2, 0, ORANGE)                   # robot_orange
draw_counter_box(draw, CELL_SIZE * 3, 0)                     # counter_box

# Row 1: key, thought_bubble, bell, lock_closed
draw_key(draw, 0, CELL_SIZE)
draw_thought_bubble(draw, CELL_SIZE, CELL_SIZE)
draw_bell(draw, CELL_SIZE * 2, CELL_SIZE)
draw_lock(draw, CELL_SIZE * 3, CELL_SIZE, closed=True)

# Row 2: lock_open, zzz, exclamation, checkmark
draw_lock(draw, 0, CELL_SIZE * 2, closed=False)
draw_zzz(draw, CELL_SIZE, CELL_SIZE * 2)
draw_exclamation(draw, CELL_SIZE * 2, CELL_SIZE * 2)
draw_checkmark(draw, CELL_SIZE * 3, CELL_SIZE * 2)

# Row 3: empty for now
# (could add more sprites later)

img.save(os.path.join(out_dir, "sprites_4x4.png"))
print(f"  Saved: {out_dir}/sprites_4x4.png")

# ── Generate digits atlas (10x1 = 320x32) ─────────────────────────────
print("Generating digits_10x1.png...")
DIGIT_SIZE = 32
digits_img = Image.new("RGBA", (DIGIT_SIZE * 10, DIGIT_SIZE), TRANSPARENT)
digits_draw = ImageDraw.Draw(digits_img)

try:
    digit_font = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 28)
except:
    digit_font = ImageFont.load_default()

for i in range(10):
    x = i * DIGIT_SIZE + DIGIT_SIZE // 2
    y = DIGIT_SIZE // 2
    digits_draw.text((x, y), str(i), fill=WHITE, font=digit_font, anchor="mm")

digits_img.save(os.path.join(out_dir, "digits_10x1.png"))
print(f"  Saved: {out_dir}/digits_10x1.png")

print("Done!")
