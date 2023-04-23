# import numpy as np
# import pandas as pd
import threading
import time
import pyautogui as pyauto
import re
import wx
import wx.adv
import wx.grid
import wx.lib.expando
import wx.lib.scrolledpanel
from pynput import keyboard


def variable_names_in(input_string):
    """Return variable in string between {{~ and ~}} syntax"""
    variables = re.findall(r'(?<={{~)(.*?)(?=~}})', input_string)
    return variables


def change_font(widget, size=None, family=None, style=None, weight=None, color=None):
    # set default parameters
    size = size if size is not None else 9
    family = family if family is not None else wx.DEFAULT
    style = style if style is not None else wx.NORMAL
    weight = weight if weight is not None else wx.NORMAL

    widget.SetFont(wx.Font(size, family, style, weight))

    if color is not None:
        widget.SetForegroundColour(color)


class PlaceholderTextCtrl(wx.TextCtrl):
    """Placeholder text ctrl."""

    def __init__(self, *args, **kwargs):
        self.default_text = kwargs.pop('placeholder', '')  # remove default text parameter
        wx.TextCtrl.__init__(self, *args, **kwargs)
        self.on_unfocus(None)

        def textctrl_tab_trigger_nav(event):
            """Function to process tab keypresses and trigger panel navigation."""
            if event.GetKeyCode() == wx.WXK_TAB:
                event.EventObject.Navigate()
            event.Skip()

        self.Bind(wx.EVT_KEY_DOWN, textctrl_tab_trigger_nav)
        self.Bind(wx.EVT_SET_FOCUS, self.on_focus)
        self.Bind(wx.EVT_KILL_FOCUS, self.on_unfocus)

    def on_focus(self, _):
        self.SetForegroundColour(wx.BLACK)
        if self.GetValue() == self.default_text:
            self.SetValue('')

    def on_unfocus(self, _):
        if self.GetValue().strip() == '':
            self.SetValue(self.default_text)
            self.SetForegroundColour(3 * (120,))


class VariablesDialog(wx.Dialog):
    """Dialog to verify license."""

    def __init__(self, parent):
        wx.Dialog.__init__(self, parent, title=f'Variables', name='variables_dialog')
        self.SetIcon(wx.Icon('data/salesforce.ico', wx.BITMAP_TYPE_ICO))
        self.SetBackgroundColour('white')
        self.license_panel = wx.Panel(self)

        self.margin_y = 10
        self.margin_x = 15

        self.vbox = wx.BoxSizer(wx.VERTICAL)  # ------------------------------------------------------------------------

        self.hbox_logo_name_version = wx.BoxSizer(wx.HORIZONTAL)

        # add rescaled logo image
        png = wx.Image('data/salesforce.png', wx.BITMAP_TYPE_PNG).Scale(60, 60, quality=wx.IMAGE_QUALITY_HIGH)
        self.logo_img = wx.StaticBitmap(self, wx.ID_ANY, wx.Bitmap(png))
        self.hbox_logo_name_version.Add(self.logo_img, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        self.vbox_name_version = wx.BoxSizer(wx.VERTICAL)

        # add program name text
        self.program_name = wx.StaticText(self, label='Variables')
        change_font(self.program_name, size=16, color=3 * (20,))
        self.vbox_name_version.Add(self.program_name, 0, wx.ALIGN_CENTER_HORIZONTAL)

        # add program version text
        self.program_version = wx.StaticText(self, label=f'2021')
        change_font(self.program_version, size=10, style=wx.ITALIC, color=3 * (80,))
        self.vbox_name_version.Add(self.program_version, 0, wx.ALIGN_CENTER_HORIZONTAL)

        self.hbox_logo_name_version.Add(self.vbox_name_version, 0, wx.ALIGN_CENTER_VERTICAL)
        self.vbox.Add(self.hbox_logo_name_version, 0, wx.ALIGN_CENTER_HORIZONTAL | wx.SOUTH, 10)

        # add input field for the license key
        self.hbox_key_input = wx.BoxSizer(wx.HORIZONTAL)

        self.hbox_key_input.Add(wx.StaticText(self.license_panel, label=f'First Name'), 0,
                                wx.ALIGN_CENTER_VERTICAL | wx.WEST, 20)

        self.hbox_key_input.AddSpacer(10)

        self.license_key_input = PlaceholderTextCtrl(self.license_panel, wx.ID_ANY, placeholder='Key', size=(280, -1),
                                                     style=wx.TE_PROCESS_ENTER | wx.TE_CENTRE)
        self.Bind(wx.EVT_TEXT_ENTER, self.activate, self.license_key_input)
        self.hbox_key_input.Add(self.license_key_input, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 20)
        self.vbox.Add(self.hbox_key_input, 0, wx.ALIGN_CENTER_HORIZONTAL | wx.SOUTH, 25)

        # add buttons
        self.button_array = wx.StdDialogButtonSizer()

        self.button_array.AddStretchSpacer()
        self.activate_btn = wx.Button(self.license_panel, label='Activate')
        self.activate_btn.Bind(wx.EVT_BUTTON, self.activate)
        self.button_array.Add(self.activate_btn)
        self.button_array.AddSpacer(5)
        self.close_btn = wx.Button(self.license_panel, label='Close')
        self.close_btn.Bind(wx.EVT_BUTTON, lambda event: self.Destroy())
        self.button_array.Add(self.close_btn)

        self.vbox.Add(self.button_array, 0, wx.EXPAND | wx.SOUTH, 10)

        self.vbox_outer = wx.BoxSizer(wx.VERTICAL)
        self.vbox_outer.AddSpacer(self.margin_y)  # north margin
        self.vbox_outer.Add(self.vbox, 0, wx.ALIGN_CENTER_HORIZONTAL | wx.EAST | wx.WEST, self.margin_x)

        # display frame
        self.license_panel.SetSizerAndFit(self.vbox_outer)
        self.vbox_outer.SetSizeHints(self)
        self.activate_btn.SetFocus()
        self.Center()
        self.Show()

    def activate(self, _):
        print('Activated')


class MonitorThread(threading.Thread):
            def __init__(self, thread_parent, macros):
                """Init Worker Thread Class."""
                threading.Thread.__init__(self, daemon=True)
                self.parent = thread_parent
                self.macros = macros
                self.released_keys = ''
                self.keyboard_controller = keyboard.Controller()

            def run(self):
                """Run worker thread."""

                def process_keystroke(key):
                    """
                    Process keystroke press or release for keyboard listener for ListenerThread instances.

                    key: parameter passed by pynput listener identifying the key
                    key_pressed: parameter passed to determine if key pressed or released
                        True: key is pressed
                        False: key is released
                    """

                    # replace numberpad virtual keycodes with strings of the number digits
                    for number in range(10):  # loop through 0-9 and replace keycodes starting at <96>
                        key = str(key).replace(f'<{96 + number}>', str(number))

                    # replace numberpad period with string
                    key = str(key).replace('<110>', '.')

                    # strip single quotes and lower
                    key = str(key).strip('\'').lower()

                    # eliminate 'key.' and make substitutions for backslash (\), single quote ('), and right ctrl (should be replaced with left)
                    key = key.replace('key.', '').replace('\\\\', '\\').replace('\"\'\"', '\'')

                    self.released_keys += (key)
                    self.released_keys = self.released_keys[-20:]  # limit stored released keys to 20
                    print(self.released_keys, [trigger for trigger in self.macros.keys()], [trigger in self.released_keys for trigger in self.macros.keys()])

                    for trigger in self.macros.keys():
                        if trigger in self.released_keys:
                            self.released_keys = ''  # clear stored keys

                            # MainFrame(self.parent, macros)

                            # wx.MessageDialog(None,
                            #                  f'Macro Data Error\n\nThe macro data one or more macro triggers and/or content that are not strings.\nPlease double check the provided macro data, especially the element.',
                            #                  'Macro Data Must in String Format', wx.ICON_ERROR).ShowModal()

                            VariablesDialog(self.parent)

                            # for ii in range(len(trigger)):
                            #     self.keyboard_controller.press(keyboard.Key.backspace)
                            #     time.sleep(0.01)
                            #
                            # self.keyboard_controller.type(self.macros[trigger])
                            # time.sleep(0.01)

                self.key_listener = keyboard.Listener(on_press=process_keystroke)
                try:
                    self.key_listener.join()
                except RuntimeError:
                    self.key_listener.start()

            def abort(self):
                """Abort worker thread."""
                # Method for use by main thread to signal an abort
                self.key_listener.stop()


class MainFrame(wx.Frame):
    """Main frame to close application."""

    def __init__(self, parent, macros):
        wx.Frame.__init__(self, parent, title=f'Salesforce Macro Automation', name='main_frame')
        self.SetIcon(wx.Icon('data/salesforce.ico', wx.BITMAP_TYPE_ICO))  # assign icon
        self.SetBackgroundColour('white')  # set background color

        self.hbox_top = wx.BoxSizer(wx.HORIZONTAL)

        # add back button
        self.back_btn = wx.Button(self, label='Start', style=wx.BORDER_NONE | wx.BU_EXACTFIT)

        self.back_btn.Bind(wx.EVT_BUTTON, lambda event: self.start())

        self.hbox_top.Add(self.back_btn, 0, wx.ALIGN_CENTER_VERTICAL)

        self.SetSizer(self.hbox_top)

        self.Show()

    def start(self):
        self.monitor_thread = MonitorThread(self, macros)
        self.monitor_thread.start()




class App(wx.App):
    """Main App to Contain Data."""

    def __init__(self, macros):
        wx.App.__init__(self)

        # check that macros array elements are strings
        for macro in macros:
            for element in macro:
                if not isinstance(element, str):
                    print(element)
                    wx.MessageDialog(None,
                                     f'Macro Data Error\n\nThe macro data one or more macro triggers and/or content that are not strings.\nPlease double check the provided macro data, especially the element \'{element}\'.',
                                     'Macro Data Must in String Format', wx.ICON_ERROR).ShowModal()
                    wx.Exit()

        MainFrame(None, macros)


if __name__ == '__main__':
    ### MACROS
    # Add macros here in a list of lists with {{~VAR~}} formatting for any dynamic variables.
    # Limit macro triggers to less than 20 alphanumeric characters and avoid modifier keys or words
    macros = {
        '`': '',
        '//init': 'Hello, my name is Noah. This is a test. Your name is {{~First Name~}} {{~Last Name~}}.',

        '//test': 'Hello hello hello.'
    }

    app = App(macros)
    app.MainLoop()