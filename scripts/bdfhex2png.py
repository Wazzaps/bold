#!/usr/bin/env python3
import sys
import PIL.Image


def bits(data):
    for byte in data:
        for bit in range(8):
            yield (byte >> (8 - bit)) & 1


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print('Usage: cat <FONT.bdf> | scripts/bdf2hex.pl | xxd -ps -r | scripts/bdfhex2png.py <FONT PNG>')
        exit(1)

    image_dest = sys.argv[1]
    image = PIL.Image.new('RGB', (144, 272))
    pixels = image.load()
    data = bits(sys.stdin.buffer.read())

    for _ in range(8*16):
        # Swallow one char for some reason
        next(data)

    for row in range(2, 16):
        for col in range(16):
            for y in range(16):
                for x in range(8):
                    color = (255, 255, 255) if next(data) else (0, 0, 0)
                    pixels[col*9 + x, row*17 + y] = color

    image.save(image_dest)
    # open(f'src/fonts/{font_name}.binfont', 'wb').write(bytearray(data))

