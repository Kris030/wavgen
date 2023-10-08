#!/bin/env python3
from matplotlib.widgets import Slider
import matplotlib.pyplot as plt
from watchfiles import watch
import numpy as np
import argparse
import wave
import sys

parser = argparse.ArgumentParser(description='Plot da wave')
parser.add_argument('--input_file', '-i', default='test.wav', help='the wav file ya retard')
args = parser.parse_args()

def f():
	print("f")
	spf = wave.open(args.input_file, 'r')

	# Extract Raw Audio from Wav File
	signal = spf.readframes(-1)
	signal = np.frombuffer(signal, dtype=np.int16)

	# If Stereo
	if spf.getnchannels() != 1:
		print('Just mono files')
		sys.exit(0)

	return signal / 0x7FFF
to_plot = f()

fig, ax = plt.subplots()

ax.set_title('Wave')
ax.set_xlabel('Sample')
ax.set_ylabel('Amplitude')

plt.subplots_adjust(bottom=0.25)

plt.plot(to_plot)

spos = Slider(
	plt.axes([0.1, 0.05, 0.8, 0.03], facecolor='yellow'),
	'Pos',
	0.0, 1.0,
	valinit=0
)

zoom = Slider(
	plt.axes([0.1, 0.1, 0.8, 0.03], facecolor='yellow'),
	'Zoom',
	0.0, 1.0,
	valinit=0.995
)

def update(_):
	p = len(to_plot) * spos.val
	z = len(to_plot) * (1 - zoom.val) / 2
	ax.axis([
		p - z,
		p + z,
		-1, 1,
	])
	fig.canvas.draw_idle()

update(None)

spos.on_changed(update)
zoom.on_changed(update)

fig.canvas.manager.set_window_title('Wave')
plt.show(block=False)

for changes in watch(args.input_file, rust_timeout=200, yield_on_timeout=True):
	if not plt.fignum_exists(1):
		break

	# plt.pause(0.1)
	if len(changes) != 0:
		to_plot = f()

		fig.clear()
		plt.plot(to_plot)

	fig.canvas.draw_idle()
	fig.canvas.start_event_loop(0.3)
