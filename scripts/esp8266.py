import serial
import time

# initialization and open the port

# possible timeout values:
#    1. None: wait forever, block call
#    2. 0: non-blocking mode, return immediately


ser = serial.Serial()
# ser.port = "/dev/ttyUSB0"
ser.port = "/dev/cu.usbserial-1410"
# ser.port = "/dev/ttyS2"
ser.baudrate = 115200
ser.bytesize = serial.EIGHTBITS  # number of bits per bytes
ser.parity = serial.PARITY_NONE  # set parity check: no parity
ser.stopbits = serial.STOPBITS_ONE  # number of stop bits
# ser.timeout = None          #block read
ser.timeout = 1  # non-block read
# ser.timeout = 2              #timeout block read
ser.xonxoff = False  # disable software flow control
ser.rtscts = False  # disable hardware (RTS/CTS) flow control
ser.dsrdtr = False  # disable hardware (DSR/DTR) flow control
ser.writeTimeout = 2  # timeout for write

try:
    ser.open()
except Exception as e:
    print("error open serial port: " + str(e))
    exit()

ser.flushInput()  # flush input buffer, discarding all its contents
ser.flushOutput()  # flush output buffer, aborting current output
# and discard all that is in buffer


"""
ATE0

AT+UART_CUR?
> +UART_CUR:115273,8,1,0,1

AT+UART_CUR=115200,8,1,0,0


AT+RFPOWER?
<  +RFPOWER:78

AT+CWQAP
> 断开

AT+CWMODE=1
1-3
1=station
2=ap
3=ap+station


AT+CWLAP
<
<  +CWLAP:(3,"cjyy",-39,"90:12:34:d4:e4:aa",6)
<  +CWLAP:(3,"",-40,"90:12:34:d4:e4:ab",6)
<  +CWLAP:(3,"Galaxy Note10+ 5G574e",-58,"4a:eb:62:15:3a:e2",11)
<  +CWLAP:(3,"feather",-65,"04:d9:f5:c4:93:98",11)

AT+CWJAP?
<  No AP
or
<  +CWJAP:"feather","04:d9:f5:c4:93:98",11,-69,0,0,0


AT+CWJAP="feather","-------"
<
<  I (2053728) wifi: state: 0 -> 2 (b0)
<  I (2053733) wifi: state: 2 -> 3 (0)
<  I (2053740) wifi: state: 3 -> 5 (10)
<  WIFI CONNECTED
<
<  WIFI GOT IP

AT+CWDHCP?
<  +CWDHCP:3

AT+CIPSTA?
<  +CIPSTA:ip:"192.168.1.9"
<  +CIPSTA:gateway:"192.168.1.1"
<  +CIPSTA:netmask:"255.255.0.0"

AT+CWHOSTNAME?
<  +CWHOSTNAME:LWIP
主机名

AT+PING="baidu.com"
<  +PING:203

AT+CIPDOMAIN="baidu.com"
<  +CIPDOMAIN:220.181.38.148

AT+CIPSTATUS
<  STATUS:2
连接状态
"""

# nc -vv -u -l -p 2000
cmd = 'AT+CIPSTART="UDP","192.168.1.198",2000,1002,2'

cmd = 'AT+CIPSTATUS'  # 获得连接id=0

# cmd = 'AT+CIPSEND=10\r\n0123456789'
# cmd = 'AT+CIPSENDBUF=10\r\n01234567891'

#cmd = 'AT+GMR'

# cmd = 'AT+RESTORE'  # 出厂设置

# cmd = 'AT+SYSLOG'

# cmd = 'AT+GMR'

# use station mode
#cmd = 'AT+CWMODE=1'

# scan ap
# cmd = 'AT+CWLAP'

# Http get
# cmd = 'AT+HTTPCLIENT=2,0,"http://httpbin.org/ip","httpbin.org","/ip",1'

ser.write(cmd.encode() + b"\r\n")
print("=> {}".format(cmd))

time.sleep(0.5)  # give the serial port sometime to receive the data

while True:
    response = ser.readline()
    print("< ", response.replace(b'\r\n', b'').decode())

    if response.strip().decode('ascii') in ['OK', 'ERROR']:
        break


ser.close()