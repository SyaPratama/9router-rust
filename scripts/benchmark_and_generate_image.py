import os
import sys
import json
import math
from PIL import Image, ImageDraw, ImageFont

def hex_to_rgb(hex_str):
    hex_str = hex_str.lstrip('#')
    return tuple(int(hex_str[i:i+2], 16) for i in (0, 2, 4))

def draw_rounded_rect(draw, bbox, radius, fill=None, outline=None, width=1):
    x0, y0, x1, y1 = bbox
    draw.rounded_rectangle([x0, y0, x1, y1], radius=radius, fill=fill, outline=outline, width=width)

def create_obsidian_benchmark_image(output_path="images/benchmark-dark.png"):
    width, height = 1600, 960
    
    # Exact Obsidian & Mint Brand Palette from hero.svg
    c_bg = hex_to_rgb("#070A0F")          # Obsidian Space
    c_card_bg = hex_to_rgb("#0C1118")     # Midnight Panel
    c_card_border = hex_to_rgb("#1B2532") # Border
    c_card_inner = hex_to_rgb("#0A0F16")  # Row surface
    c_track_bg = hex_to_rgb("#111822")    # Bar track
    
    # Brand Accents
    c_mint = hex_to_rgb("#00F0B5")        # Headroom Mint (Primary)
    c_cyan = hex_to_rgb("#65D8FF")        # Cyan Glow (Secondary)
    c_white = hex_to_rgb("#FFFFFF")       # Pure White
    c_slate = hex_to_rgb("#98A4B3")       # Slate Muted
    c_dim = hex_to_rgb("#6B7888")         # Dim text
    c_bar_gray = hex_to_rgb("#2C3B4E")    # Neutral comparison bar
    c_bar_dark = hex_to_rgb("#1F2937")    # Low comparison bar
    
    img = Image.new('RGB', (width, height), c_bg)
    draw = ImageDraw.Draw(img)

    # 1. Subtle Background Grid (40x40 step, faint mint)
    for x in range(0, width, 40):
        draw.line([(x, 0), (x, height)], fill=(10, 18, 22), width=1)
    for y in range(0, height, 40):
        draw.line([(0, y), (width, y)], fill=(10, 18, 22), width=1)

    # 2. Subtle Radial Glow (top center)
    glow_center_x, glow_center_y = width // 2, 140
    for r in range(400, 0, -20):
        alpha = int(14 * (1 - r / 400.0))
        glow_color = (0, int(240 * (alpha / 255.0)), int(181 * (alpha / 255.0)))
        draw.ellipse([glow_center_x - r, glow_center_y - int(r * 0.4), glow_center_x + r, glow_center_y + int(r * 0.4)], outline=None, fill=None)

    # Fonts
    font_bold_path = "C:/Windows/Fonts/segoeuib.ttf"
    font_reg_path = "C:/Windows/Fonts/segoeui.ttf"
    font_mono_path = "C:/Windows/Fonts/consola.ttf"
    
    font_badge = ImageFont.truetype(font_bold_path, 12)
    font_hero_title = ImageFont.truetype(font_bold_path, 34)
    font_hero_sub = ImageFont.truetype(font_reg_path, 16)
    font_kpi_num = ImageFont.truetype(font_bold_path, 34)
    font_kpi_lbl = ImageFont.truetype(font_bold_path, 12)
    font_kpi_sub = ImageFont.truetype(font_reg_path, 13)
    font_card_title = ImageFont.truetype(font_bold_path, 18)
    font_card_badge = ImageFont.truetype(font_bold_path, 10)
    font_row_label = ImageFont.truetype(font_bold_path, 15)
    font_row_val = ImageFont.truetype(font_bold_path, 16)
    font_row_tag = ImageFont.truetype(font_bold_path, 12)
    font_footer = ImageFont.truetype(font_reg_path, 12)

    # 3. Header Section with Geometric Vector Gateway Icon (NO EMOJIS)
    icon_x, icon_y = 60, 42
    draw_rounded_rect(draw, [icon_x, icon_y, icon_x + 44, icon_y + 44], radius=8, fill=c_card_bg, outline=c_card_border, width=1)
    draw.ellipse([icon_x + 18, icon_y + 18, icon_x + 26, icon_y + 26], fill=c_mint)
    draw.line([(icon_x + 22, icon_y + 8), (icon_x + 22, icon_y + 16)], fill=c_cyan, width=2)
    draw.line([(icon_x + 22, icon_y + 28), (icon_x + 22, icon_y + 36)], fill=c_cyan, width=2)
    draw.line([(icon_x + 8, icon_y + 22), (icon_x + 16, icon_y + 22)], fill=c_cyan, width=2)
    draw.line([(icon_x + 28, icon_y + 22), (icon_x + 36, icon_y + 22)], fill=c_cyan, width=2)
    draw.ellipse([icon_x + 20, icon_y + 6, icon_x + 24, icon_y + 10], fill=c_white)
    draw.ellipse([icon_x + 20, icon_y + 34, icon_x + 24, icon_y + 38], fill=c_white)
    draw.ellipse([icon_x + 6, icon_y + 20, icon_x + 10, icon_y + 24], fill=c_white)
    draw.ellipse([icon_x + 34, icon_y + 20, icon_x + 38, icon_y + 24], fill=c_white)

    # Title & Subtitle
    draw.text((120, 42), "9Router-Rust Benchmark Performance", fill=c_white, font=font_hero_title)
    draw.text((120, 86), "Live TCP loopback socket measurements vs 9Router (Next.js) & OmniRoute on 4-vCPU host @ concurrency 50", fill=c_slate, font=font_hero_sub)

    # Architecture Pill Badge (top right)
    badge_w, badge_h = 240, 36
    bx0, by0 = width - 60 - badge_w, 46
    draw_rounded_rect(draw, [bx0, by0, bx0 + badge_w, by0 + badge_h], radius=18, fill=c_card_bg, outline=c_card_border, width=1)
    draw.ellipse([bx0 + 16, by0 + 14, bx0 + 24, by0 + 22], fill=c_mint)
    draw.text((bx0 + 34, by0 + 10), "NATIVE AXUM + TOKIO", fill=c_mint, font=font_badge)

    # 4. Top KPI Cards (Throughput, Memory, Latency)
    kpis = [
        {
            "num": "29.3x",
            "label": "THROUGHPUT MULTIPLIER",
            "sub": "15,408 req/s vs 525 req/s legacy",
            "accent": c_mint,
            "icon": "speed"
        },
        {
            "num": "14.4 MB",
            "label": "IDLE MEMORY (RSS)",
            "sub": "89% reduction vs 131 MB Next.js",
            "accent": c_cyan,
            "icon": "chip"
        },
        {
            "num": "2.58 ms",
            "label": "MEDIAN LATENCY (p50)",
            "sub": "33.5x faster than 86.4 ms legacy",
            "accent": c_white,
            "icon": "clock"
        }
    ]

    kpi_w = 466
    kpi_h = 105
    kpi_y = 135

    for idx, k in enumerate(kpis):
        kx = 60 + idx * (kpi_w + 31)
        draw_rounded_rect(draw, [kx, kpi_y, kx + kpi_w, kpi_y + kpi_h], radius=8, fill=c_card_bg, outline=c_card_border, width=1)
        draw.line([(kx, kpi_y), (kx + kpi_w, kpi_y)], fill=k["accent"], width=2)
        
        # Clean vector icon (NO EMOJI)
        ix, iy = kx + 24, kpi_y + 26
        if k["icon"] == "speed":
            draw.arc([ix, iy, ix + 26, iy + 26], start=140, end=400, fill=k["accent"], width=2)
            draw.line([(ix + 13, iy + 13), (ix + 20, iy + 7)], fill=k["accent"], width=2)
            draw.ellipse([ix + 11, iy + 11, ix + 15, iy + 15], fill=k["accent"])
        elif k["icon"] == "chip":
            draw.rectangle([ix + 4, iy + 4, ix + 22, iy + 22], outline=k["accent"], fill=c_bg, width=2)
            for p in range(8, 20, 4):
                draw.line([(ix + p, iy), (ix + p, iy + 4)], fill=k["accent"], width=1)
                draw.line([(ix + p, iy + 22), (ix + p, iy + 26)], fill=k["accent"], width=1)
                draw.line([(ix, iy + p), (ix + 4, iy + p)], fill=k["accent"], width=1)
                draw.line([(ix + 22, iy + p), (ix + 26, iy + p)], fill=k["accent"], width=1)
            draw.rectangle([ix + 9, iy + 9, ix + 17, iy + 17], fill=k["accent"])
        elif k["icon"] == "clock":
            draw.ellipse([ix + 2, iy + 4, ix + 24, iy + 26], outline=k["accent"], width=2)
            draw.line([(ix + 13, iy + 15), (ix + 13, iy + 9)], fill=k["accent"], width=2)
            draw.line([(ix + 13, iy + 15), (ix + 18, iy + 15)], fill=k["accent"], width=2)
            draw.line([(ix + 10, iy + 1), (ix + 16, iy + 1)], fill=k["accent"], width=2)

        # Numbers and labels
        draw.text((kx + 68, kpi_y + 16), k["num"], fill=k["accent"], font=font_kpi_num)
        draw.text((kx + 68, kpi_y + 60), k["label"], fill=c_slate, font=font_kpi_lbl)
        draw.text((kx + 68, kpi_y + 78), k["sub"], fill=c_dim, font=font_kpi_sub)

    # 5. Main Comparison Panels (3 Columns)
    cards = [
        {
            "title": "THROUGHPUT",
            "badge": "HIGHER IS BETTER",
            "badge_color": c_mint,
            "badge_bg": (0, 36, 28),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "15,408 req/s",
                    "ratio": 1.0,
                    "bar_color": c_mint,
                    "tag": "29.3x",
                    "tag_color": c_mint,
                    "is_winner": True
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "525 req/s",
                    "ratio": 0.034,
                    "bar_color": c_bar_gray,
                    "tag": "1.0x baseline",
                    "tag_color": c_dim,
                    "is_winner": False
                },
                {
                    "name": "OmniRoute",
                    "val_str": "354 req/s",
                    "ratio": 0.023,
                    "bar_color": c_bar_dark,
                    "tag": "0.67x",
                    "tag_color": c_dim,
                    "is_winner": False
                }
            ]
        },
        {
            "title": "IDLE MEMORY (RSS)",
            "badge": "LOWER IS BETTER",
            "badge_color": c_cyan,
            "badge_bg": (8, 30, 42),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "14.4 MB",
                    "ratio": 14.4 / 424.0,
                    "bar_color": c_cyan,
                    "tag": "89% LESS",
                    "tag_color": c_cyan,
                    "is_winner": True
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "131.0 MB",
                    "ratio": 131.0 / 424.0,
                    "bar_color": c_bar_gray,
                    "tag": "9.1x larger",
                    "tag_color": c_dim,
                    "is_winner": False
                },
                {
                    "name": "OmniRoute",
                    "val_str": "424.0 MB",
                    "ratio": 1.0,
                    "bar_color": c_bar_dark,
                    "tag": "29.4x larger",
                    "tag_color": c_dim,
                    "is_winner": False
                }
            ]
        },
        {
            "title": "MEDIAN LATENCY (P50)",
            "badge": "LOWER IS BETTER",
            "badge_color": c_cyan,
            "badge_bg": (8, 30, 42),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "2.58 ms",
                    "ratio": 2.58 / 123.1,
                    "bar_color": c_cyan,
                    "tag": "33.5x FASTER",
                    "tag_color": c_cyan,
                    "is_winner": True
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "86.4 ms",
                    "ratio": 86.4 / 123.1,
                    "bar_color": c_bar_gray,
                    "tag": "33.5x slower",
                    "tag_color": c_dim,
                    "is_winner": False
                },
                {
                    "name": "OmniRoute",
                    "val_str": "123.1 ms",
                    "ratio": 1.0,
                    "bar_color": c_bar_dark,
                    "tag": "47.7x slower",
                    "tag_color": c_dim,
                    "is_winner": False
                }
            ]
        }
    ]

    card_y = 265
    card_w = 466
    card_h = 600

    for c_idx, card in enumerate(cards):
        cx = 60 + c_idx * (card_w + 31)
        
        # Outer Card Box
        draw_rounded_rect(draw, [cx, card_y, cx + card_w, card_y + card_h], radius=8, fill=c_card_bg, outline=c_card_border, width=1)
        
        # Card Header
        draw.text((cx + 24, card_y + 24), card["title"], fill=c_white, font=font_card_title)
        
        # Direction Badge ("HIGHER IS BETTER" / "LOWER IS BETTER")
        badge_w = 120
        bx0 = cx + card_w - badge_w - 24
        draw_rounded_rect(draw, [bx0, card_y + 22, bx0 + badge_w, card_y + 46], radius=4, fill=card["badge_bg"], outline=card["badge_color"], width=1)
        draw.text((bx0 + 12, card_y + 27), card["badge"], fill=card["badge_color"], font=font_card_badge)

        # Draw 3 Comparison Rows
        row_start_y = card_y + 80
        row_spacing = 160

        for r_idx, r in enumerate(card["rows"]):
            ry = row_start_y + r_idx * row_spacing
            
            # Row Container
            draw_rounded_rect(draw, [cx + 18, ry, cx + card_w - 18, ry + 138], radius=6, fill=c_card_inner, outline=c_card_border, width=1)
            
            # Name and Tag
            name_color = c_white if r["is_winner"] else c_slate
            draw.text((cx + 34, ry + 16), r["name"], fill=name_color, font=font_row_label)
            
            # Tag Pill on right
            tw = len(r["tag"]) * 8 + 18
            tag_x = cx + card_w - 34 - tw
            pill_bg = (0, 40, 30) if r["is_winner"] else (20, 28, 38)
            draw_rounded_rect(draw, [tag_x, ry + 14, tag_x + tw, ry + 36], radius=4, fill=pill_bg)
            draw.text((tag_x + 9, ry + 17), r["tag"], fill=r["tag_color"], font=font_row_tag)

            # Metric Value String
            val_color = r["bar_color"] if r["is_winner"] else c_slate
            draw.text((cx + 34, ry + 46), r["val_str"], fill=val_color, font=font_row_val)

            # Progress Bar Background Track
            bar_track_box = [cx + 34, ry + 84, cx + card_w - 34, ry + 110]
            draw_rounded_rect(draw, bar_track_box, radius=4, fill=c_track_bg, outline=c_card_border, width=1)

            # Progress Bar Active Fill
            max_bar_w = (cx + card_w - 34) - (cx + 34)
            bar_fill_w = max(int(r["ratio"] * max_bar_w), 12)
            bar_fill_box = [cx + 34, ry + 84, cx + 34 + bar_fill_w, ry + 110]
            draw_rounded_rect(draw, bar_fill_box, radius=4, fill=r["bar_color"])
            
            # Vertical white notch line at the end of winner bar (like hero.svg)
            if r["is_winner"]:
                draw.rectangle([cx + 34 + bar_fill_w - 4, ry + 84, cx + 34 + bar_fill_w, ry + 110], fill=c_white)

    # 6. Clean Footer (No "verified", no emojis)
    draw.rectangle([60, height - 60, 64, height - 44], fill=c_mint)
    footer_left = "Tested on /api/health across 1,000 requests @ concurrency 50 on 4-vCPU host. Reproducible with cargo test and 9router bench"
    draw.text((74, height - 57), footer_left, fill=c_dim, font=font_footer)
    
    footer_right = "Obsidian & Mint Architecture · 100% Native Rust"
    right_bbox = font_footer.getbbox(footer_right)
    right_w = right_bbox[2] - right_bbox[0]
    draw.text((width - 60 - right_w, height - 57), footer_right, fill=c_dim, font=font_footer)

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    img.save(output_path, "PNG", quality=95)
    print(f"--> Obsidian & Mint benchmark image created: {output_path}")

if __name__ == "__main__":
    create_obsidian_benchmark_image("images/benchmark-dark.png")
