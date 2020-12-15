from PIL import Image, ImageDraw, ImageFont
import PIL.features

print('libraqm:', PIL.features.check('raqm'))


size = (320, 16)

"""
FORMAT = "RGB"
BG = (255, 255, 255)
FG = (0, 0, 0)
"""

FORMAT = '1'
BG = 0
FG = 1

YOFF = 0  # or -1


CHARS = "气温湿度照压卧槽艹牛逼数据"


im = Image.new(FORMAT, size, BG)

# font = ImageFont.truetype("sarasa-mono-sc-nerd-light.ttf", size=16, index=0)
# font = ImageFont.truetype("sarasa-mono-sc-nerd-regular.ttf", size=16, index=0)
font = ImageFont.truetype("Unibit.ttf", size=16, index=0)
# font = ImageFont.truetype("zpix.ttf", size=12, index=0)


draw = ImageDraw.Draw(im)

draw.text((0, YOFF), CHARS, font=font, fill=FG, language='zh-CN')
im.save('font.png')
im.show()


draw.rectangle([(0, 0), size], fill=BG)


# NOTE, char 127 is replaced with '°'
ASCII = '\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f !"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~°'

charmap = []

for i, c in enumerate(ASCII):
    if c.isprintable():
        draw.text((0, YOFF), c, font=font, fill=FG)
    else:
        draw.text((0, YOFF), "  ", font=font, fill=FG)

    for y in range(16):
        v = 0
        for x in range(0, 8):
            b = im.getpixel((x, y))
            v = (v << 1) + b

        charmap.append(v)

    draw.rectangle([(0, 0), size], fill=BG)

# ascii done

# print(len(charmap))

for i, c in enumerate(CHARS):
    draw.text((0, YOFF), c, font=font, fill=FG)

    for y in range(16):
        v = 0
        for x in range(0, 16):
            b = im.getpixel((x, y))
            v = (v << 1) + b

        charmap.append(v >> 8)
        charmap.append(v & 0xFF)

    draw.rectangle([(0, 0), size], fill=BG)


with open('font.raw', 'wb') as fp:
    fp.write(bytes(charmap))
