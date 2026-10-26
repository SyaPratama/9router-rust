import os
import sys
import json
from PIL import Image, ImageDraw, ImageFont

def hex_to_rgb(hex_str):
    hex_str = hex_str.lstrip('#')
    return tuple(int(hex_str[i:i+2], 16) for i in (0, 2, 4))

def draw_rounded_rect(draw, bbox, radius, fill=None, outline=None, width=1):
    x0, y0, x1, y1 = bbox
    draw.rounded_rectangle([x0, y0, x1, y1], radius=radius, fill=fill, outline=outline, width=width)

def create_colorful_benchmark_image(output_path="images/benchmark-dark.png"):
    width, height = 1600, 960
    
    # Modern obsidian palette
    c_bg = hex_to_rgb("#0A0E17")
    c_card_bg = hex_to_rgb("#111827")
    c_card_border = hex_to_rgb("#1F2937")
    c_card_inner = hex_to_rgb("#161F30")
    
    # Brand and accent colors
    c_rust_orange = hex_to_rgb("#F97316")
    c_emerald = hex_to_rgb("#10B981")
    c_cyan = hex_to_rgb("#06B6D4")
    c_indigo = hex_to_rgb("#6366F1")
    c_slate = hex_to_rgb("#64748B")
    c_rose = hex_to_rgb("#F43F5E")
    c_amber = hex_to_rgb("#F59E0B")
    
    # Typography colors
    c_text_bright = hex_to_rgb("#F9FAFB")
    c_text_muted = hex_to_rgb("#9CA3AF")
    c_text_dim = hex_to_rgb("#6B7280")
    
    img = Image.new('RGB', (width, height), c_bg)
    draw = ImageDraw.Draw(img)

    # Fonts
    font_bold_path = "C:/Windows/Fonts/segoeuib.ttf"
    font_reg_path = "C:/Windows/Fonts/segoeui.ttf"
    
    font_badge = ImageFont.truetype(font_bold_path, 13)
    font_hero_title = ImageFont.truetype(font_bold_path, 36)
    font_hero_sub = ImageFont.truetype(font_reg_path, 17)
    font_kpi_num = ImageFont.truetype(font_bold_path, 34)
    font_kpi_lbl = ImageFont.truetype(font_bold_path, 13)
    font_kpi_sub = ImageFont.truetype(font_reg_path, 13)
    font_card_title = ImageFont.truetype(font_bold_path, 20)
    font_card_badge = ImageFont.truetype(font_bold_path, 11)
    font_row_label = ImageFont.truetype(font_bold_path, 15)
    font_row_val = ImageFont.truetype(font_bold_path, 15)
    font_row_tag = ImageFont.truetype(font_bold_path, 12)
    font_footer = ImageFont.truetype(font_reg_path, 13)

    # 1. Header Section
    badge_box = [60, 42, 280, 72]
    draw_rounded_rect(draw, badge_box, radius=15, fill=hex_to_rgb("#064E3B"), outline=c_emerald, width=1)
    draw.text((78, 48), "⚡ NATIVE RUST ENGINE 1.0", fill=hex_to_rgb("#6EE7B7"), font=font_badge)

    draw.text((60, 85), "9Router-Rust Benchmark Performance", fill=c_text_bright, font=font_hero_title)
    draw.text((60, 136), "Live TCP loopback measurements vs legacy 9Router (Next.js) & OmniRoute on 4-vCPU host @ concurrency 50", fill=c_text_muted, font=font_hero_sub)

    # 2. Hero KPI Cards (Top Row)
    kpis = [
        {
            "num": "29.3x",
            "label": "THROUGHPUT MULTIPLIER",
            "sub": "15,408 req/s vs 525 req/s legacy",
            "color": c_rust_orange
        },
        {
            "num": "14.4 MB",
            "label": "IDLE MEMORY FOOTPRINT",
            "sub": "89% reduction vs 131 MB Next.js",
            "color": c_emerald
        },
        {
            "num": "2.58 ms",
            "label": "MEDIAN LATENCY (p50)",
            "sub": "33.5x faster than 86.4 ms legacy",
            "color": c_cyan
        }
    ]

    kpi_w = 466
    kpi_h = 105
    kpi_y = 175
    for idx, k in enumerate(kpis):
        kx = 60 + idx * (kpi_w + 31)
        draw_rounded_rect(draw, [kx, kpi_y, kx + kpi_w, kpi_y + kpi_h], radius=10, fill=c_card_bg, outline=c_card_border, width=1)
        # Accent bar on top of KPI card
        draw_rounded_rect(draw, [kx, kpi_y, kx + kpi_w, kpi_y + 4], radius=2, fill=k["color"])
        
        draw.text((kx + 22, kpi_y + 16), k["num"], fill=k["color"], font=font_kpi_num)
        draw.text((kx + 22, kpi_y + 60), k["label"], fill=c_text_bright, font=font_kpi_lbl)
        draw.text((kx + 22, kpi_y + 80), k["sub"], fill=c_text_dim, font=font_kpi_sub)

    # 3. Main Comparison Panels (3 Columns)
    cards = [
        {
            "title": "Throughput",
            "badge": "HIGHER IS BETTER",
            "badge_color": c_emerald,
            "badge_bg": hex_to_rgb("#064E3B"),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "15,408 req/s",
                    "ratio": 1.0,
                    "bar_color": c_rust_orange,
                    "tag": "🏆 29.3x",
                    "tag_color": c_rust_orange
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "525 req/s",
                    "ratio": 0.034,
                    "bar_color": c_indigo,
                    "tag": "1.0x baseline",
                    "tag_color": c_slate
                },
                {
                    "name": "OmniRoute",
                    "val_str": "354 req/s",
                    "ratio": 0.023,
                    "bar_color": c_slate,
                    "tag": "0.67x",
                    "tag_color": c_slate
                }
            ]
        },
        {
            "title": "Idle Memory (RSS)",
            "badge": "LOWER IS BETTER",
            "badge_color": c_cyan,
            "badge_bg": hex_to_rgb("#083344"),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "14.4 MB",
                    "ratio": 14.4 / 424.0,
                    "bar_color": c_emerald,
                    "tag": "✨ 89% LESS",
                    "tag_color": c_emerald
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "131.0 MB",
                    "ratio": 131.0 / 424.0,
                    "bar_color": c_amber,
                    "tag": "9.1x larger",
                    "tag_color": c_slate
                },
                {
                    "name": "OmniRoute",
                    "val_str": "424.0 MB",
                    "ratio": 1.0,
                    "bar_color": c_rose,
                    "tag": "29.4x larger",
                    "tag_color": c_slate
                }
            ]
        },
        {
            "title": "Median Latency (p50)",
            "badge": "LOWER IS BETTER",
            "badge_color": c_cyan,
            "badge_bg": hex_to_rgb("#083344"),
            "rows": [
                {
                    "name": "9Router-Rust",
                    "val_str": "2.58 ms",
                    "ratio": 2.58 / 123.1,
                    "bar_color": c_cyan,
                    "tag": "⚡ 33.5x FASTER",
                    "tag_color": c_cyan
                },
                {
                    "name": "9Router (Next.js)",
                    "val_str": "86.4 ms",
                    "ratio": 86.4 / 123.1,
                    "bar_color": c_indigo,
                    "tag": "33.5x slower",
                    "tag_color": c_slate
                },
                {
                    "name": "OmniRoute",
                    "val_str": "123.1 ms",
                    "ratio": 1.0,
                    "bar_color": c_slate,
                    "tag": "47.7x slower",
                    "tag_color": c_slate
                }
            ]
        }
    ]

    card_y = 305
    card_w = 466
    card_h = 560

    for c_idx, card in enumerate(cards):
        cx = 60 + c_idx * (card_w + 31)
        
        # Outer Card Box
        draw_rounded_rect(draw, [cx, card_y, cx + card_w, card_y + card_h], radius=12, fill=c_card_bg, outline=c_card_border, width=1)
        
        # Card Header
        draw.text((cx + 24, card_y + 24), card["title"], fill=c_text_bright, font=font_card_title)
        
        # Direction Badge ("HIGHER IS BETTER" / "LOWER IS BETTER")
        badge_w = 125
        bx0 = cx + card_w - badge_w - 24
        draw_rounded_rect(draw, [bx0, card_y + 24, bx0 + badge_w, card_y + 48], radius=6, fill=card["badge_bg"], outline=card["badge_color"], width=1)
        draw.text((bx0 + 10, card_y + 29), card["badge"], fill=card["badge_color"], font=font_card_badge)

        # Draw Comparison Rows
        row_start_y = card_y + 85
        row_spacing = 150

        for r_idx, r in enumerate(card["rows"]):
            ry = row_start_y + r_idx * row_spacing
            
            # Row Container
            draw_rounded_rect(draw, [cx + 18, ry, cx + card_w - 18, ry + 130], radius=8, fill=c_card_inner, outline=c_card_border, width=1)
            
            # Name and Tag
            draw.text((cx + 34, ry + 16), r["name"], fill=c_text_bright, font=font_row_label)
            
            # Tag Pill on right
            tw = len(r["tag"]) * 8 + 16
            tag_x = cx + card_w - 34 - tw
            draw_rounded_rect(draw, [tag_x, ry + 14, tag_x + tw, ry + 36], radius=4, fill=hex_to_rgb("#1E293B"))
            draw.text((tag_x + 8, ry + 17), r["tag"], fill=r["tag_color"], font=font_row_tag)

            # Metric Value String
            draw.text((cx + 34, ry + 46), r["val_str"], fill=c_text_bright, font=font_row_val)

            # Progress Bar Background Track
            bar_track_box = [cx + 34, ry + 82, cx + card_w - 34, ry + 104]
            draw_rounded_rect(draw, bar_track_box, radius=6, fill=hex_to_rgb("#1F2937"))

            # Progress Bar Active Fill
            max_bar_w = (cx + card_w - 34) - (cx + 34)
            bar_fill_w = max(int(r["ratio"] * max_bar_w), 12)
            bar_fill_box = [cx + 34, ry + 82, cx + 34 + bar_fill_w, ry + 104]
            draw_rounded_rect(draw, bar_fill_box, radius=6, fill=r["bar_color"])

    # 4. Footer Note
    footer_text = "Verified run published in raw-results.json (2026-10-26). Reproduce locally: cargo test --release --test benchmark_test && 9router bench"
    draw.text((60, height - 55), footer_text, fill=c_text_dim, font=font_footer)

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    img.save(output_path, "PNG", quality=95)
    print(f"--> Colorful modern benchmark image created: {output_path}")

if __name__ == "__main__":
    create_colorful_benchmark_image("images/benchmark-dark.png")
