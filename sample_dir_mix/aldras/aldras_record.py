"""Aldras module containing recording objects"""
import wx
from modules.aldras_core import PlaceholderTextCtrl
from modules.aldras_settings import import_settings


def create_record_options(parent_frame, settings_frame=False):

    def record_some_sleep_pressed():
        if not settings_frame:
            some_sleep_thresh.Enable(True)

    def not_some_sleep_pressed():
        if not settings_frame:
            some_sleep_thresh.Enable(False)

    settings = import_settings()

    hbox_options = wx.BoxSizer(wx.HORIZONTAL)

    some_sleep_thresh = 0.2

    # recording pause input
    sleep_sizer = wx.StaticBoxSizer(wx.StaticBox(parent_frame, label='Record pause between actions'), wx.VERTICAL)  # ---------

    record_no_sleep = wx.RadioButton(parent_frame, wx.ID_ANY, 'No pauses')
    record_no_sleep.Bind(wx.EVT_RADIOBUTTON, lambda event: not_some_sleep_pressed())
    sleep_sizer.Add(record_no_sleep, 0, wx.ALL, 4)

    record_all_sleep = wx.RadioButton(parent_frame, wx.ID_ANY, 'All pauses over 0.5')
    record_all_sleep.Bind(wx.EVT_RADIOBUTTON, lambda event: not_some_sleep_pressed())
    sleep_sizer.Add(record_all_sleep, 0, wx.ALL, 4)

    some_sleep_hbox = wx.BoxSizer(wx.HORIZONTAL)
    some_sleep_hbox.AddSpacer(4)
    record_some_sleep = wx.RadioButton(parent_frame, wx.ID_ANY, 'Pauses over')
    record_some_sleep.Bind(wx.EVT_RADIOBUTTON, lambda event: record_some_sleep_pressed())
    some_sleep_hbox.Add(record_some_sleep, 0, wx.ALIGN_CENTER_VERTICAL)

    if settings_frame:
        some_sleep_thresh = wx.TextCtrl(parent_frame, wx.ID_ANY, value=str(settings['Record pause over duration']), size=wx.Size(50, -1),
                                                style=wx.TE_CENTER, name='some_sleep_thresh')
    else:
        some_sleep_thresh = PlaceholderTextCtrl(parent_frame, wx.ID_ANY, placeholder=str(settings['Record pause over duration']), size=wx.Size(50, -1),
                                                style=wx.TE_CENTER, name='some_sleep_thresh')
        some_sleep_thresh.Enable(settings['Record pause'] == 'Pauses over')
    some_sleep_hbox.Add(some_sleep_thresh)

    some_sleep_hbox.Add(wx.StaticText(parent_frame, label='  sec   '), 0, wx.ALIGN_CENTER_VERTICAL)
    sleep_sizer.Add(some_sleep_hbox, 0, wx.BOTTOM, 5)

    hbox_options.Add(sleep_sizer, 0, wx.ALL, 5)
    # ------------------------------------------------------------------------------------------------------
    hbox_options.AddSpacer(5)

    # recording method input
    record_mthd = wx.RadioBox(parent_frame, label='Method', choices=['Overwrite', 'Append'], majorDimension=1, style=wx.RA_SPECIFY_COLS, name='Record method')
    record_mthd.SetItemToolTip(0, 'Erase existing data')
    record_mthd.SetItemToolTip(1, 'Add to end of existing data')
    hbox_options.Add(record_mthd, 0, wx.ALL, 5)

    return hbox_options