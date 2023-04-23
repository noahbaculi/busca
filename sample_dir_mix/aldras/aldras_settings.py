"""Aldras module containing settings objects"""
import json
import os

import wx

from modules.aldras_core import CharValidator, directory_chooser

settings_possibilities = {
    'Number of recent workflows displayed': [str(ii) for ii in list(range(0, 6))],  # zero to five
    'Freeze method': ['None', 'Click', 'Ctrl', 'Click or ctrl'],
    'Number of hotkeys': ['2', '3', '4'],
    'Notifications': ['Banners', 'Windows'],
    'Record pause': ['No pauses', 'All pauses over 0.5', 'Pauses over'],
    'Record method': ['Overwrite', 'Append'],
}


def validate_settings(settings_unvalidated):
    # factory settings for reference comparison (double quotes used rather than single quote convention due to desire to allow copy-paste between this dictionary and the .json file)
    factory_settings = {
        "Number of recent workflows displayed": 3,
        "Workflow folder": "",
        "Freeze method": "Ctrl",
        "Number of hotkeys": 3,
        "Large lines number": 100,
        "Notifications": "Banners",
        "Record pause": "Pauses over",
        "Record pause over duration": 5.0,
        "Record method": "Append",
        "Execute pause between commands": 1.0,
        "Execute pause between commands checked": "True",
        "Execute mouse command duration": 0.5,
        "Execute mouse command duration checked": "True",
        "Interval between text character outputs": 0.02,
        "Interval between text character outputs checked": "True"
    }

    # settings validation lambda functions
    settings_validation = {
        'Number of recent workflows displayed': lambda x: str(x) in settings_possibilities[
            'Number of recent workflows displayed'],
        "Workflow folder": lambda dir_path: os.path.exists(dir_path),
        'Freeze method': lambda x: x.lower() in [y.lower() for y in settings_possibilities['Freeze method']],
        'Number of hotkeys': lambda x: 2 <= x <= 4,
        'Large lines number': lambda x: 15 <= x <= 200,
        'Notifications': lambda x: x.lower() in [y.lower() for y in settings_possibilities['Notifications']],
        'Record pause': lambda x: x.lower() in [y.lower() for y in settings_possibilities['Record pause']],
        'Record pause over duration': lambda x: x > 0,
        'Record method': lambda x: x.lower() in [y.lower() for y in settings_possibilities['Record method']],
        'Execute pause between commands': lambda x: x > 0,
        'Execute pause between commands checked': lambda x: x in [True, False],
        'Execute mouse command duration': lambda x: x > 0,
        'Execute mouse command duration checked': lambda x: x in [True, False],
        'Interval between text character outputs': lambda x: x > 0,
        'Interval between text character outputs checked': lambda x: x in [True, False]
    }

    settings = dict()
    for key in factory_settings:  # loop through the factory settings and attempt to parse imported setting if available

        # determine type of factory settings to cast imported settings
        if factory_settings[key] in ['True', 'False']:
            cast_type = bool
        else:
            cast_type = type(factory_settings[key])

        try:
            if settings_validation[key](
                    cast_type(settings_unvalidated[key])):  # if the cast imported setting is validated
                settings[key] = cast_type(settings_unvalidated[key])  # set equal to the cast imported setting

                if key != 'Workflow folder' and isinstance(settings[key],
                                                           str):  # do not modify captilization of workflow folder path (just for aesthetic reasons)
                    settings[key] = settings[key].capitalize()  # capitalize setting if string
            else:
                raise ValueError
        except (KeyError, ValueError):
            settings[key] = cast_type(factory_settings[key])

    if settings['Workflow folder'] == '':
        default_save_folder_dlg = wx.MessageDialog(None,
                                                   'Please select the default directory where Workflow files should be saved.',
                                                   'Choose default save location', wx.YES_NO | wx.ICON_INFORMATION)
        default_save_folder_dlg.SetYesNoLabels('Select', 'Exit')  # rename 'Yes' and 'No' labels to 'Select' and 'Exit'

        if default_save_folder_dlg.ShowModal() == wx.ID_YES:
            settings['Workflow folder'] = directory_chooser(None)
            save_settings(settings)
        else:
            raise SystemExit

        default_save_folder_dlg.Destroy()

    return settings


def import_settings():
    # open data/settings.json file to import settings, otherwise create empty dictionary to use factory settings
    try:
        with open('data/settings.json', 'r') as json_file:
            imported_settings = validate_settings(json.load(json_file))

    except (FileNotFoundError, json.decoder.JSONDecodeError) as error:
        if isinstance(error, FileNotFoundError):
            wx.MessageDialog(None, 'The \'settings.json\' file could not be located and has been reconstructed.',
                             'Missing settings.json file', wx.OK | wx.ICON_INFORMATION).ShowModal()
        if isinstance(error, json.decoder.JSONDecodeError):
            wx.MessageDialog(None, 'The \'settings.json\' file could not be decoded and has been reconstructed.',
                             'Corrupt settings.json file', wx.OK | wx.ICON_INFORMATION).ShowModal()

        # reconstruct settings.json file with factory settings
        with open('data/settings.json', 'w') as json_file:
            imported_settings = validate_settings(dict())
            json.dump(imported_settings, json_file, indent=4)

    return imported_settings


def save_settings(settings):
    settings = validate_settings(settings)
    with open('data/settings.json', 'w') as json_file:
        json.dump(settings, json_file, indent=4)


def open_settings(parent_window):
    settings_dlg = SettingsDialog(parent_window)
    if settings_dlg.ShowModal() == wx.ID_OK:
        settings_old = import_settings()
        save_settings(settings_dlg.settings)

        # prompt user to restart Aldras if parameters were changed affecting SelectionFrame or EditFrame
        difference = False
        for parameter in ['Number of recent workflows displayed', 'Workflow folder', 'Number of hotkeys',
                          'Large lines number',
                          'Notifications']:
            if settings_old[parameter] != settings_dlg.settings[parameter]:
                difference = True
                break

        if difference:
            settings_restart_dlg = wx.MessageDialog(settings_dlg,
                                                    'Aldras may need to be restarted for changes to be fully applied',
                                                    'Restart Aldras to apply changes', wx.YES_NO | wx.ICON_WARNING)
            settings_restart_dlg.SetYesNoLabels('Restart', 'Later')  # rename 'Yes' and 'No' labels to 'Restart' and 'Later'

            if settings_restart_dlg.ShowModal() == wx.ID_YES:
                # relaunch SelectionFrame
                if parent_window.GetName() == 'edit_frame':  # close EditFrame and save workflow if needed
                    parent_window.Close()
                    parent_window.parent.restart()
                elif parent_window.GetName() == 'selection_frame':
                    parent_window.restart()

            settings_restart_dlg.Destroy()

    settings_dlg.Destroy()


class SettingsDialog(wx.Dialog):
    """Main frame to select workflow."""

    def __init__(self, parent):
        wx.Dialog.__init__(self, parent, title=f'Settings', name='settings_frame')
        self.SetBackgroundColour('white')
        if parent:
            self.SetIcon(wx.Icon(parent.software_info.icon, wx.BITMAP_TYPE_ICO))  # assign icon

        margin = 30

        def setting_cb(parameter, parent_window=None):
            if not parent_window:
                parent_window = panel

            return wx.ComboBox(parent_window, value=str(self.settings[parameter]),
                               choices=settings_possibilities[parameter],
                               style=wx.CB_READONLY)

        self.settings = import_settings()
        self.settings_as_imported = self.settings.copy()

        static_boxsizer_inner_padding = 5
        static_boxsizer_outer_spacing = 12

        # create sizers
        vbox_outer = wx.BoxSizer(wx.VERTICAL)
        vbox_main = wx.BoxSizer(wx.VERTICAL)
        vbox_container = wx.BoxSizer(wx.VERTICAL)

        panel = wx.Panel(self)

        # add rescaled image
        png = wx.Image('data/settings.png', wx.BITMAP_TYPE_PNG).Scale(120, 120,
                                                                      quality=wx.IMAGE_QUALITY_HIGH)
        self.logo_img = wx.StaticBitmap(panel, wx.ID_ANY, wx.Bitmap(png))
        vbox_container.Add(self.logo_img, 0, wx.ALIGN_CENTER_HORIZONTAL | wx.NORTH | wx.SOUTH, round(margin / 2))

        workflow_folder_sizer = wx.StaticBoxSizer(wx.StaticBox(panel, wx.ID_ANY, 'Workflow Folder'), wx.VERTICAL)  # ---
        workflow_folder_hbox = wx.BoxSizer(wx.HORIZONTAL)

        self.workflow_folder_st = wx.StaticText(panel, label='')
        workflow_folder_hbox.Add(self.workflow_folder_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        self.set_workflow_folder_text()

        workflow_folder_hbox.AddStretchSpacer()

        workflow_folder_chooser = wx.DirPickerCtrl(panel, path=self.settings['Workflow folder'],
                                                   style=wx.DIRP_DIR_MUST_EXIST)
        workflow_folder_chooser.Bind(wx.EVT_DIRPICKER_CHANGED,
                                     lambda event: self.setting_change(event.GetPath(), 'Workflow folder'))
        workflow_folder_hbox.Add(workflow_folder_chooser, 0, wx.ALIGN_CENTER_VERTICAL)

        workflow_folder_sizer.Add(workflow_folder_hbox, 0, wx.EXPAND | wx.ALL, static_boxsizer_inner_padding)
        vbox_container.Add(workflow_folder_sizer, 0, wx.EXPAND | wx.SOUTH, static_boxsizer_outer_spacing)  # -----------

        #

        selection_sizer = wx.StaticBoxSizer(wx.StaticBox(panel, wx.ID_ANY, 'Workflow Selection'), wx.VERTICAL)  # ------

        num_recent_workflows_hbox = wx.BoxSizer(wx.HORIZONTAL)
        num_recent_workflows_st = wx.StaticText(panel, label='Number of recent workflows displayed:')
        num_recent_workflows_hbox.Add(num_recent_workflows_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        num_recent_workflows_cb = setting_cb('Number of recent workflows displayed')
        num_recent_workflows_cb.Bind(wx.EVT_COMBOBOX, lambda event: self.setting_change(event.GetString(),
                                                                                        'Number of recent workflows displayed'))
        num_recent_workflows_hbox.Add(num_recent_workflows_cb, 0, wx.ALIGN_CENTER_VERTICAL)

        selection_sizer.Add(num_recent_workflows_hbox, 0, wx.ALL, static_boxsizer_inner_padding)
        vbox_container.Add(selection_sizer, 0, wx.EXPAND | wx.SOUTH, static_boxsizer_outer_spacing)  # -----------------

        #

        mouse_monitor_sizer = wx.StaticBoxSizer(wx.StaticBox(panel, wx.ID_ANY, 'Mouse Monitor'), wx.VERTICAL)  # -------

        mouse_monitor_freeze_mthd_hbox = wx.BoxSizer(wx.HORIZONTAL)

        mouse_monitor_freeze_mthd_st = wx.StaticText(panel, label='Freeze method:')
        mouse_monitor_freeze_mthd_hbox.Add(mouse_monitor_freeze_mthd_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)

        mouse_monitor_freeze_mthd_cb = setting_cb('Freeze method')
        mouse_monitor_freeze_mthd_cb.Bind(wx.EVT_COMBOBOX,
                                          lambda event: self.setting_change(event.GetString(), 'Freeze method'))
        mouse_monitor_freeze_mthd_hbox.Add(mouse_monitor_freeze_mthd_cb, 0, wx.ALIGN_CENTER_VERTICAL)

        mouse_monitor_sizer.Add(mouse_monitor_freeze_mthd_hbox, 0, wx.ALL, static_boxsizer_inner_padding)
        vbox_container.Add(mouse_monitor_sizer, 0, wx.EXPAND | wx.SOUTH, static_boxsizer_outer_spacing)  # -------------

        #

        editor_collpane = wx.CollapsiblePane(panel,
                                             label=' Editor')  # -------------------------------------------------
        editor_collpane.GetChildren()[0].SetBackgroundColour(wx.WHITE)  # set button and label background
        editor_collpane.GetChildren()[0].Bind(wx.EVT_KILL_FOCUS,
                                              lambda event: None)  # prevents flickering when focus is killed
        vbox_container.Add(editor_collpane, 0, wx.GROW | wx.SOUTH, static_boxsizer_outer_spacing)
        editor_panel = editor_collpane.GetPane()

        editor_sizer = wx.StaticBoxSizer(wx.StaticBox(editor_panel), wx.VERTICAL)

        editor_number_of_hotkeys_hbox = wx.BoxSizer(wx.HORIZONTAL)

        editor_number_of_hotkeys_st = wx.StaticText(editor_panel, label='Number of hotkeys:')
        editor_number_of_hotkeys_hbox.Add(editor_number_of_hotkeys_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        editor_number_of_hotkeys_cb = setting_cb('Number of hotkeys', editor_panel)
        editor_number_of_hotkeys_cb.Bind(wx.EVT_COMBOBOX,
                                         lambda event: self.setting_change(event.GetString(), 'Number of hotkeys'))
        editor_number_of_hotkeys_hbox.Add(editor_number_of_hotkeys_cb, 0, wx.ALIGN_CENTER_VERTICAL)
        editor_sizer.Add(editor_number_of_hotkeys_hbox, 0, wx.ALL, static_boxsizer_inner_padding)

        editor_number_many_lines_hbox = wx.BoxSizer(wx.HORIZONTAL)
        editor_number_many_lines_st = wx.StaticText(editor_panel,
                                                    label='Number of large number of lines to prompt warning:')
        editor_number_many_lines_hbox.Add(editor_number_many_lines_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        editor_number_many_lines_cb = wx.TextCtrl(editor_panel, style=wx.TE_CENTRE,
                                                  value=str(self.settings['Large lines number']), size=wx.Size(40, -1),
                                                  validator=CharValidator('only_integer', self))
        editor_number_many_lines_cb.SetMaxLength(3)
        editor_number_many_lines_cb.Bind(wx.EVT_TEXT,
                                         lambda event: self.setting_change(event.GetString(), 'Large lines number'))
        editor_number_many_lines_hbox.Add(editor_number_many_lines_cb, 0, wx.ALIGN_CENTER_VERTICAL)
        editor_sizer.Add(editor_number_many_lines_hbox, 0, wx.ALL, static_boxsizer_inner_padding)

        editor_notifications_hbox = wx.BoxSizer(wx.HORIZONTAL)
        editor_notifications_st = wx.StaticText(editor_panel, label='Notifications:')
        editor_notifications_hbox.Add(editor_notifications_st, 0, wx.ALIGN_CENTER_VERTICAL | wx.EAST, 10)
        editor_notifications_cb = setting_cb('Notifications', editor_panel)
        editor_notifications_cb.Bind(wx.EVT_COMBOBOX,
                                     lambda event: self.setting_change(event.GetString(), 'Notifications'))
        editor_notifications_hbox.Add(editor_notifications_cb, 0, wx.ALIGN_CENTER_VERTICAL)
        editor_sizer.Add(editor_notifications_hbox, 0, wx.ALL, static_boxsizer_inner_padding)

        editor_panel.SetSizer(editor_sizer)
        editor_sizer.SetSizeHints(editor_panel)  # ---------------------------------------------------------------------

        #

        record_collpane = wx.CollapsiblePane(panel, label=' Record Options')  # ----------------------------------------
        record_collpane.GetChildren()[0].SetBackgroundColour(wx.WHITE)  # set button and label background
        record_collpane.GetChildren()[0].Bind(wx.EVT_KILL_FOCUS,
                                              lambda event: None)  # prevents flickering when focus is killed
        vbox_container.Add(record_collpane, 0, wx.GROW | wx.SOUTH, static_boxsizer_outer_spacing)
        record_panel = record_collpane.GetPane()

        record_sizer = wx.StaticBoxSizer(wx.StaticBox(record_panel), wx.VERTICAL)

        from modules.aldras_record import create_record_options
        record_options_sizer = create_record_options(record_panel, settings_frame=True)

        # bind parameter changes to setting_change()
        for record_pause_option in settings_possibilities['Record pause']:
            self.FindWindowByLabel(record_pause_option).Bind(wx.EVT_RADIOBUTTON, lambda event: self.setting_change(
                event.GetEventObject().GetLabel(), 'Record pause'))

        self.FindWindowByLabel(self.settings['Record pause']).SetValue(True)

        self.FindWindowByName('some_sleep_thresh').SetValue(str(self.settings['Record pause over duration']))
        self.FindWindowByName('some_sleep_thresh').Bind(wx.EVT_TEXT, lambda event: self.setting_change(
            event.GetEventObject().GetValue(), 'Record pause over duration'))

        self.FindWindowByName('Record method').SetSelection(
            self.FindWindowByName('Record method').FindString(self.settings['Record method']))
        self.FindWindowByName('Record method').Bind(wx.EVT_RADIOBOX, lambda event: self.setting_change(
            event.GetEventObject().GetString(event.GetEventObject().GetSelection()), 'Record method'))

        record_sizer.Add(record_options_sizer, 0, wx.ALL, static_boxsizer_inner_padding)
        record_panel.SetSizer(record_sizer)
        record_sizer.SetSizeHints(record_panel)  # ---------------------------------------------------------------------

        #

        execute_collpane = wx.CollapsiblePane(panel,
                                              label=' Execute Options')  # ----------------------------------------
        execute_collpane.GetChildren()[0].SetBackgroundColour(wx.WHITE)  # set button and label background
        execute_collpane.GetChildren()[0].Bind(wx.EVT_KILL_FOCUS,
                                               lambda event: None)  # prevents flickering when focus is killed
        vbox_container.Add(execute_collpane, 0, wx.GROW | wx.SOUTH, static_boxsizer_outer_spacing)
        execute_panel = execute_collpane.GetPane()

        execute_sizer = wx.StaticBoxSizer(wx.StaticBox(execute_panel), wx.VERTICAL)

        from modules.aldras_execute import create_execute_options
        execute_options_sizer = create_execute_options(execute_panel, settings_frame=True)

        for setting_name in ['Execute pause between commands', 'Execute pause between commands checked',
                             'Execute mouse command duration', 'Execute mouse command duration checked',
                             'Interval between text character outputs',
                             'Interval between text character outputs checked']:
            widget = self.FindWindowByName(setting_name)

            if isinstance(widget, wx.CheckBox):  # set true or false, not string
                widget.SetValue(self.settings[setting_name])
                widget.Bind(wx.EVT_CHECKBOX,
                            lambda event, setting=setting_name: self.setting_change(event.GetEventObject().GetValue(),
                                                                                    setting))
            elif isinstance(widget, wx.TextCtrl):
                widget.SetValue(str(self.settings[setting_name]))
                widget.Bind(wx.EVT_TEXT,
                            lambda event, setting=setting_name: self.setting_change(event.GetEventObject().GetValue(),
                                                                                    setting))

        execute_sizer.Add(execute_options_sizer, 0, wx.ALL, static_boxsizer_inner_padding)

        execute_panel.SetSizer(execute_sizer)
        execute_sizer.SetSizeHints(execute_panel)  # -------------------------------------------------------------------

        #

        panel.SetSizer(vbox_container)
        vbox_main.Add(panel, 0, wx.EXPAND)

        # add buttons
        btns = self.CreateSeparatedButtonSizer(wx.OK | wx.CANCEL)
        vbox_main.Add(btns, 0, wx.EXPAND | wx.ALL, 5)

        self.FindWindowById(wx.ID_OK).Enable(False)

        vbox_outer.AddStretchSpacer()
        vbox_outer.Add(vbox_main, 0, wx.EXPAND | wx.WEST | wx.EAST, margin)
        vbox_outer.AddSpacer(margin / 2)
        vbox_outer.AddStretchSpacer()

        self.SetSizerAndFit(vbox_outer)

        self.Center()

    def setting_change(self, value, setting):
        self.settings[setting] = value
        self.settings = validate_settings(self.settings)

        if setting == 'Workflow folder':
            self.set_workflow_folder_text()

        if self.settings != self.settings_as_imported:
            self.FindWindowById(wx.ID_OK).Enable()  # enable OK button if changes
        else:
            self.FindWindowById(wx.ID_OK).Enable(False)  # disable OK button if no changes

    def set_workflow_folder_text(self):
        """Set label of workflow folder static text"""
        workflow_folder_text = self.settings['Workflow folder']

        # cut out middle characters if path is too long
        workflow_folder_text_char_limit = 45
        if len(workflow_folder_text) > workflow_folder_text_char_limit:
            workflow_folder_text_start = workflow_folder_text[:round(workflow_folder_text_char_limit / 2) - 3]
            workflow_folder_text_end = workflow_folder_text[-round(workflow_folder_text_char_limit / 2) + 2:]
            workflow_folder_text = f'{workflow_folder_text_start} ... {workflow_folder_text_end}'

        self.workflow_folder_st.SetLabel(workflow_folder_text)