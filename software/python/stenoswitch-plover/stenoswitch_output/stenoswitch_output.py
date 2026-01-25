#!/usr/bin/env python3

# Outputs plover translations as ASCII to a temp file.
# Based on https://plover.readthedocs.io/en/latest/plugin-dev/extensions.html.
# Also based on https://github.com/Anodynous/stenogotchi/.

import os
import plover.key_combo

# Store pressed or not characters as unused keycodes
def pressed_to_code(pressed: bool):
  if pressed:
      return '\x81'
  else:
      return '\x8D'

KEYNAME_TRANSFORM = key_combo.add_modifiers_aliases(key_combo.KEYNAME_TO_CHAR)

def keyname_transform(key_name: str):
    KEYNAME_TRANSFORM.get_or_default(key_name, key_name)

class StrokeLogger:
  def __init__(self, engine):
    self.engine = engine
    self.output_file = None

  def start(self):
    os.mkfifo("/tmp/plover_output")
    self.output_file = open("/tmp/plover_output")

    self.engine.hook_connect("send_string", self.on_send_string)
    self.engine.hook_connect("send_backspaces", self.on_send_backspaces)
    self.engine.hook_connect("send_key_combination", self.on_send_key_combination)

  def stop(self):
    self.engine.hook_connect("send_string", self.on_send_string)
    self.engine.hook_connect("send_backspaces", self.on_send_backspaces)
    self.engine.hook_connect("send_key_combination", self.on_send_key_combination)
    self.output_file.close()

  def on_send_string(self, string):
    self.output_file.write(string)
    print(string, file=self.output_file)

  def on_send_backspaces(self, num):
    self.output_file.write("".join(['\b'] * num))
    print(string, file=self.output_file)

  def on_send_key_combination(self, combo):
    self.output_file.write("".join([[code, self.pressed_to_code(pressed)] for (code, pressed) in plover.key_combo.parse_key_combo(combo, keyname_transform)]))
