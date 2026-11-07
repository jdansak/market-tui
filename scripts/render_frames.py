import json
import os
from PIL import Image, ImageDraw, ImageFont

TITLE_MAP = {
    "01_overview_btc_candlesticks": "market_tui — Market Overview (Japanese Candlesticks & Live Volume)",
    "02_timeline_scrubber": "market_tui — Timeline Scrubber & Historical Candle Inspector",
    "03_overview_nvda_1month_full_width": "market_tui — 1M Timeframe: Full-Width Dynamic Bar Spacing",
    "04_overview_aapl_solid_line": "market_tui — Dense Continuous Solid Line Chart (AAPL)",
    "05_options_max_pain": "market_tui — Options Chain & Max Pain Strike Visualizer",
    "06_correlation_heatmap": "market_tui — Cross-Asset Correlation Heatmap & Rolling Beta",
    "07_sec_insider_trades": "market_tui — SEC Form 4 Smart Money & Insider Trades Radar",
    "08_yield_curve_recession": "market_tui — US Treasury Yield Curve & Recession Probability Radar",
}

def main():
    json_path = "assets/frames.json"
    if not os.path.exists(json_path):
        print("frames.json not found")
        return

    with open(json_path, "r", encoding="utf-8") as f:
        frames = json.load(f)

    font_path = "/System/Library/Fonts/Menlo.ttc"
    font_size = 15
    try:
        font = ImageFont.truetype(font_path, font_size)
        title_font = ImageFont.truetype(font_path, 13)
    except Exception:
        font = ImageFont.load_default()
        title_font = font

    dummy_img = Image.new("RGB", (100, 100))
    dummy_draw = ImageDraw.Draw(dummy_img)
    bbox = dummy_draw.textbbox((0, 0), "M", font=font)
    char_w = max(bbox[2] - bbox[0], 9)
    char_h = max(bbox[3] - bbox[1] + 6, 18)

    title_bar_h = 38
    padding_x = 18
    padding_y = 14

    grid_w = frames[0]["width"]
    grid_h = frames[0]["height"]

    img_w = padding_x * 2 + grid_w * char_w
    img_h = title_bar_h + padding_y * 2 + grid_h * char_h

    bg_color = (13, 17, 23)
    title_bar_color = (22, 27, 34)
    border_color = (48, 54, 61)

    rendered_images = []

    for idx, frame in enumerate(frames):
        img = Image.new("RGB", (img_w, img_h), bg_color)
        draw = ImageDraw.Draw(img)

        # Title bar background
        draw.rectangle([(0, 0), (img_w, title_bar_h)], fill=title_bar_color)
        draw.line([(0, title_bar_h), (img_w, title_bar_h)], fill=border_color, width=1)

        # Window control dots
        dot_y = title_bar_h // 2
        draw.ellipse([(16, dot_y - 6), (28, dot_y + 6)], fill=(255, 95, 86))
        draw.ellipse([(36, dot_y - 6), (48, dot_y + 6)], fill=(255, 189, 46))
        draw.ellipse([(56, dot_y - 6), (68, dot_y + 6)], fill=(39, 201, 63))

        # Title text
        raw_name = frame["name"]
        title_text = TITLE_MAP.get(raw_name, f"market_tui — {raw_name}")
        tbbox = draw.textbbox((0, 0), title_text, font=title_font)
        tw = tbbox[2] - tbbox[0]
        tx = (img_w - tw) // 2
        ty = (title_bar_h - (tbbox[3] - tbbox[1])) // 2
        draw.text((tx, ty), title_text, font=title_font, fill=(139, 155, 180))

        # Terminal outer border
        draw.rectangle([(0, 0), (img_w - 1, img_h - 1)], outline=border_color, width=1)

        # Render cells
        base_x = padding_x
        base_y = title_bar_h + padding_y

        for c in frame["cells"]:
            cx = base_x + c["x"] * char_w
            cy = base_y + c["y"] * char_h
            fg = tuple(c["fg"])
            draw.text((cx, cy), c["ch"], font=font, fill=fg)

        frame_png = f"assets/frame_{idx:02d}.png"
        img.save(frame_png)
        rendered_images.append(img)
        print(f"Rendered {frame_png} ({title_text})")

    # Save standalone showcase screenshots
    if len(rendered_images) > 0:
        rendered_images[0].save("assets/screenshot_overview.png")
    if len(rendered_images) > 4:
        rendered_images[4].save("assets/screenshot_options.png")
    if len(rendered_images) > 5:
        rendered_images[5].save("assets/screenshot_correlation.png")
    if len(rendered_images) > 7:
        rendered_images[7].save("assets/screenshot_yield_curve.png")

    durations = [3200, 2400, 2400, 2400, 3200, 3200, 3000, 3500]

    rendered_images[0].save(
        "assets/market_tui_demo.gif",
        save_all=True,
        append_images=rendered_images[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )
    print("Saved assets/market_tui_demo.gif")

if __name__ == "__main__":
    main()
