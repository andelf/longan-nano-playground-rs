from PIL import Image


# im = Image.open("./out/0879.png")
# with open("./sample.raw", 'wb') as fp:
#    fp.write(im_to_raw(im))

# RGB, 106x80


def to_rgb565(r, g, b):
    r = (r & 0b11111000) << 8
    g = (g & 0b11111100) << 3
    b = b >> 3
    val = r + g + b
    return val.to_bytes(2, 'big')


def im_to_raw(im):
    raw = []
    # im_gray = im.convert('L')
    for y in range(80):
        for x in range(106):
            r, g, b = im.getpixel((x, y))
            raw.append(to_rgb565(r, g, b))

    return b''.join(raw)


# Must use DOS 8.3 file name.
with open("./badapple.raw", "wb") as fp:
    for i in range(1, 5258 + 1):
        fname = "./out/%04d.png" % i
        print(fname)
        im = Image.open(fname)
        raw = im_to_raw(im)
        fp.write(raw)
