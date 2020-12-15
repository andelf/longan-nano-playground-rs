
from PIL import Image, ImageDraw, ImageFont


size = (320, 16)

FORMAT = "RGB"
BG = (255, 255, 255)
FG = (0, 0, 0)

FORMAT = '1'
BG= 0
FG=1


CHARS = "卧槽艹牛逼气温湿度"


im = Image.new(FORMAT, size, BG)

font = ImageFont.truetype("sarasa-mono-sc-nerd-regular.ttf", size=16, index=0)
# get a drawing context
draw = ImageDraw.Draw(im)


# draw.text((0,-1), CHARS, font=font, fill=FG)
# draw text, full opacity
# d.text((10,60), "World", font=fnt, fill=(255,255,255,255))


ASCII = '\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f !"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\x7f'

charmap = []

for i, c in enumerate(ASCII):
    if c.isprintable():
        draw.text((0,-1), c, font=font, fill=FG)
    else:
        draw.text((0,-1), "  ", font=font, fill=FG)

    for y in range(16):
        v = 0
        for x in range(0, 8):
            b = im.getpixel((x, y))
            v = (v << 1) + b

        charmap.append(v)

    draw.rectangle([(0, 0), size], fill=BG)

# ascii done

print(len(charmap))

for i, c in enumerate(CHARS):
    draw.text((0,-1), c, font=font, fill=FG)

    for y in range(16):
        v = 0
        for x in range(0, 16):
            b = im.getpixel((x, y))
            v = (v << 1) + b

        charmap.append(v >> 8)
        charmap.append(v & 0xff)

    draw.rectangle([(0, 0), size], fill=BG)



with open('font.raw', 'wb')  as fp:
    fp.write(bytes(charmap))

im.save('out.png')
# im.show()
