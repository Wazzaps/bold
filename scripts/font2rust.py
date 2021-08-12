#!/usr/bin/env python3
import sys
import PIL.Image

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print('Usage: scripts/font2rust.py <FONT PNG> <FONT NAME>')
        exit(1)
    image_source = sys.argv[1]
    font_name = sys.argv[2]
    image = PIL.Image.open(image_source)
    assert image.size == (144, 272), 'Incorrect font image dimensions'
    pixels = image.load()

    data = []
    for row in range(16):
        for col in range(16):
            for y in range(16):
                for x in range(8):
                    data.append(pixels[col*9 + x, row*17 + y][0])

    bitpack = []
    bit = 0
    for pixel in data:
        if bit == 0:
            bitpack.append(0)
        if pixel:
            bitpack[-1] |= 1 << bit
        bit = (bit + 1) % 8

    open(f'src/fonts/{font_name}.binfont', 'wb').write(bytearray(bitpack))

