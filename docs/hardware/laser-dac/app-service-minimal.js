globalThis["webpackJsonp"] = globalThis["webpackJsonp"] || [];
globalThis["webpackJsonp"].push([

    ["app-service"], {

        "appStateManager": function (e, t, r) {
            "use strict";
            (function (logger) {

                var r = {
                    globalData: {

                        // List of Bluetooth service UUIDs to connect to
                        mserviceuuids: [],
                        // List of Bluetooth characteristic UUIDs for transmitting (TX) data
                        mtxduuids: [],
                        // List of Bluetooth characteristic UUIDs for receiving (RX) data
                        mrxduuids: [],
                        // 0,1,2
                        muuidSel: 0,
                        bleManualDisCnn: !1,
                        BLEConnectionStateChangeSet: !1,
                        BluetoothAdapterOpen: false,
                        ble_device: null,
                        blu_state: 0,
                        blu_connect_stop: !1,
                        blu_connected: 0,
                        //  global "stop BLE operations" flag
                        blu_readyRec: !1,
                        blu_cnn_call_back: null,
                        blu_rec_call_back: null,
                        blu_rec_content: null,
                        blu_data_canSend: !1,
                        blu_data_cmdSending: !1,
                        blu_data_send_interval: 100,
                        deviceInfo: {},
                        cmd: {
                            curMode: 0,
                            settingData: {
                                dmx: 0,
                                ch: 0,
                                xy: 0,
                                light: 1,
                                cfg: 0,
                                lang: 0,
                                valArr: [1, 10, 10, 10, 10]
                            },
                            // prjIndex: The current project index (default 0).
                            // public: Shared settings for all projects (rdMode, runSpeed, txColor, soundVal).
                            // prjItem: An object mapping project indices (as strings) to their specific settings, each with pyMode, prjSelected (an array of 4 numbers), 
                            // and ckValues (an array, initially empty)      
                            prjData: {
                                prjIndex: 0,
                                public: {
                                    rdMode: 0, // audio trigger mode, 0 or 255
                                    runSpeed: 50,
                                    txColor: 9, //colorDisplayOrder
                                    soundVal: 20 // setting sound sensitivity
                                },
                                prjItem: {
                                    2: {
                                        pyMode: 0, // 0,255  loop playback, tick play
                                        prjSelected: [0, 0, 0, 0], /// Pattern selection (bitfield)
                                        ckValues: [] // selected items
                                    },
                                    3: {
                                        pyMode: 0,
                                        prjSelected: [0, 0, 0, 0], /// Pattern selection (bitfield)
                                        ckValues: []
                                    },
                                    5: {
                                        pyMode: 0,
                                        prjSelected: [0, 0, 0, 0], /// Pattern selection (bitfield)
                                        ckValues: []
                                    },
                                    6: {
                                        pyMode: 0,
                                        prjSelected: [0, 0, 0, 0],
                                        ckValues: []
                                    }
                                }
                            },

                        },

                        // sets the value of blu_data_send_interval in the global data object to the value passed as e.
                        // It is used to update the interval (in milliseconds) at which Bluetooth data is sent.
                        setbluDataSendInterval: function (interval) {
                            this.blu_data_send_interval = interval
                        },

                        // invokes the registered Bluetooth receive callback with the provided data, 
                        //  if a callback is set.
                        setRecCallBack: function (data) {
                            var callbackFunc = this.blu_rec_call_back;
                            null != callbackFunc && callbackFunc(data)
                        },

                        // updates the Bluetooth connection state, saves the device if connected, 
                        // and notifies any registered callback.
                        setBluCnnState: function (connectionState, isManualChange) {
                            this.blu_connected = connectionState, 2 == this.blu_connected && this.saveDevice();
                            var connectionCallback = this.blu_cnn_call_back;
                            null != connectionCallback && connectionCallback(connectionState, isManualChange)
                        },

                        setCmdMode: function (mode) {
                            this.cmd["curMode"] = mode,
                                this.cmd["prjData"].prjIndex = mode
                        },
                        getCmdData: function (commandKey) {
                            return this.cmd[commandKey]
                        },

                        //  function is a setter used to update command-related data within the cmd object. 
                        // It takes two arguments: e, which is the key indicating which section of the command data to update, 
                        // and t, which is the new data to set.
                        // If the key e is "prjData", the function updates the public property of cmd.prjData with t.public. 
                        // If t.prjIndex is not 1, it also updates the corresponding prjItem entry using the index as a string. 
                        // Additionally, it synchronizes the runSpeed and txColor properties in cmd.textData with those from 
                        // t.public. The use of void ensures the function returns undefined after these assignments,
                        //  exiting early for this case.

                        // For all other keys, the function simply assigns t to cmd[e]. If the key is "textData", 
                        // it also updates cmd.prjData.public.runSpeed and cmd.prjData.public.txColor 
                        // to match the new values in t. This ensures that related properties stay consistent across different 
                        // sections of the command data structure.

                        setCmdData: function (key, data) {
                            if ("prjData" == key) return this.cmd[key].public = data.public, 1 != data.prjIndex
                                && (this.cmd[key].prjItem[data.prjIndex + ""] = data.item),
                                this.cmd.textData.runSpeed = data.public.runSpeed,
                                void (this.cmd.textData.txColor = data.public.txColor);
                            this.cmd[key] = data, "textData" == key
                                && (this.cmd.prjData.public.runSpeed = data.runSpeed, this.cmd.prjData.public.txColor = data.txColor)
                        },

                        // restores the last used Bluetooth UUID configuration by reading a saved index 
                        // and updating the relevant UUID arrays for device communication.
                        readSetting: function () {
                            switch (this.muuidSel = this.readData("lastsel") || 0, this.muuidSel) {
                                case 0:
                                    this.mserviceuuids = ["0000FF00-0000-1000-8000-00805F9B34FB"],
                                        this.mtxduuids = ["0000FF02-0000-1000-8000-00805F9B34FB"],
                                        this.mrxduuids = "0000FF01-0000-1000-8000-00805F9B34FB";
                                    break;
                                case 1:
                                    this.mserviceuuids = ["0000FFE0-0000-1000-8000-00805F9B34FB"],
                                        this.mtxduuids = ["0000FFE1-0000-1000-8000-00805F9B34FB"],
                                        this.mrxduuids = ["0000FFE1-0000-1000-8000-00805F9B34FB"];
                                    break;
                                case 2:
                                    this.mserviceuuids = ["0000FF00-0000-1000-8000-00805F9B34FB"],
                                        this.mtxduuids = ["0000FF02-0000-1000-8000-00805F9B34FB"],
                                        this.mrxduuids = "0000FF01-0000-1000-8000-00805F9B34FB";
                                    break
                            }
                        },

                        setDeviceInfo: function (deviceType, version, userType) {
                            this.deviceInfo["deviceType"] = deviceType,
                                this.deviceInfo["version"] = version,
                                this.deviceInfo["userType"] = userType
                        },
                        getDeviceInfo: function () {
                            var e = his.deviceInfo["deviceType"];
                            "" == e && (e = 0);
                            var t = this.deviceInfo["version"];
                            "" == t && (t = 0);
                            var r = this.deviceInfo["userType"];
                            return "" == r && (r = 0), {
                                deviceType: parseInt(e),
                                version: parseInt(t),
                                userType: parseInt(r)
                            }
                        },

                        // getDeviceFeatures() returns an object indicating which features are supported by the current device, 
                        // based on its type and version.
                        getDeviceFeatures: function () {
                            var features = {
                                textStopTime: false,
                                textDecimalTime: false,
                                displayType: 0,
                                showOutDoorTips: false,
                                xyCnf: false,
                                arbPlay: false,
                                ilda: false,
                                // device supports TTL analog
                                ttlAn: false,
                                picsPlay: false,
                                textUpDown: false,
                                animationFix: false
                            },
                                deviceType = this.deviceInfo["deviceType"],
                                version = this.deviceInfo["version"];
                            // Determine device features based on deviceType and version
                            // Each feature is enabled according to specific deviceType/version rules
                            //  "00 - 02" represents a device with type 0 and version 2,
                            if (
                                (deviceType === 1 && version >= 1) ||
                                (deviceType === 0 && version >= 2) ||
                                deviceType >= 2
                            ) {
                                features.textStopTime = true;
                                features.textDecimalTime = true;
                            }

                            if (
                                (deviceType === 1 && version >= 2) ||
                                deviceType > 1
                            ) {
                                features.showOutDoorTips = true;
                            }

                            if (deviceType === 1 && version === 1) {
                                features.textModeFix01 = true;
                            }

                            if (deviceType === 2) {
                                features.xyCnf = true;
                            }

                            if (deviceType === 1 || deviceType === 2) {
                                features.ilda = true;
                                features.ttlAn = true;
                            }

                            if (deviceType >= 2 || version >= 3) {
                                features.arbPlay = true;
                            }

                            if (deviceType >= 3 || version >= 4) {
                                features.textUpDown = true;
                            }

                            if (deviceType >= 3 || version >= 5) {
                                features.picsPlay = true;
                            }

                            if (deviceType === 1) {
                                features.animationFix = true;
                            }

                            features.displayType = deviceType;
                            return features;
                        },


                        savelastsel: function (t) {
                            this.saveData("lastsel", t), logger("log", "Writelastsel ", t, " at App.vue:328")
                        },

                        // restores the last used Bluetooth device from persistent storage into the ble_device property.
                        readDevice: function () {
                            this.ble_device = this.readData("device")
                        },
                        saveDevice: function () {
                            this.saveData("device", this.ble_device)
                        },

                        clearDevice: function () {
                            this.ble_device = null, this.saveDevice()
                        },

                        createBLEConnection: function (deviceId) {
                            var r = this,
                                callbackFunc = arguments.length > 1 && void 0 !== arguments[1]
                                    ? arguments[1]
                                    : null;

                            this.blu_connected = -1,
                                uni.createBLEConnection({
                                    deviceId: deviceId,
                                    timeout: 6e3,
                                    success: function (e) {
                                        callbackFunc && callbackFunc(!0)
                                    },
                                    fail: function (h) {
                                        r.bleManualDisCnn
                                            ? callbackFunc && callbackFunc(!1)
                                            : r.doCloseBLEConnection(deviceId, (function (e) {
                                                callbackFunc && callbackFunc(!1)
                                            }))
                                    }
                                })
                        },

                        doCloseBLEConnection: function (deviceId) {
                            var callbackFunc = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null;

                            this.bleManualDisCnn = true;
                            var n = this,
                                h = setTimeout((function () {
                                    h = null, n.bleManualDisCnn = !1, callbackFunc && callbackFunc(!0)
                                }), 200);
                            uni.closeBLEConnection({
                                deviceId: deviceId,
                                success: function (t) {
                                    logger("log", "doCloseBLEConnection success", t, " at App.vue:384"), h
                                        && callbackFunc && callbackFunc(!0)
                                },
                                fail: function (t) {
                                    logger("log", "doCloseBLEConnection fail", t, " at App.vue:389"), h
                                        && callbackFunc && callbackFunc(!1)
                                },
                                complete: function () {
                                    logger("log", "doCloseBLEConnection complete", " at App.vue:394"), h
                                        && clearTimeout(h), this.bleManualDisCnn = !1
                                }
                            })
                        },
                        closeBLEConnection: function () {
                            var callbackFunc = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                            if (this.blu_connected) {
                                var device = this.ble_device;
                                device ? this.doCloseBLEConnection(device.deviceId, (function (r) {
                                    logger("log", "do callback", " at App.vue:409"),
                                        callbackFunc && callbackFunc(r)
                                })) : callbackFunc && callbackFunc(!0)
                            } else callbackFunc && callbackFunc(!0)
                        },
                        doCloseBluetoothAdapter: function () {
                            var callbackFunc = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                            this.bleOpenCloseCount--, this.BluetoothAdapterOpen = !1, uni.closeBluetoothAdapter({
                                success: function (e) {
                                    callbackFunc && callbackFunc(!0)
                                },
                                fail: function (r) {
                                    logger("log", "closeBluetoothAdapter fail", r, " at App.vue:424"),
                                        callbackFunc && callbackFunc(!1)
                                }
                            })
                        },
                        closeBluetoothAdapter: function () {
                            var callbackFunc = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                            this.BluetoothAdapterOpen ? this.doCloseBluetoothAdapter(callbackFunc) : callbackFunc && callbackFunc(!0)
                        },
                        openBluetoothAdapter: function () {
                            var t = this,
                                callbackFunc = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                            if (this.BluetoothAdapterOpen) callbackFunc && callbackFunc(!0);
                            else {
                                this.bleOpenCloseCount++;
                                var n = this;
                                uni.openBluetoothAdapter({
                                    success: function (e) {
                                        t.BluetoothAdapterOpen = !0, t.setBLEConnectionStateChange(), callbackFunc && callbackFunc(!0)
                                    },
                                    fail: function (h) {
                                        t.doCloseBluetoothAdapter(), 10001 === h.errCode
                                            && t.showModalTips(n.$t("\u8bf7\u68c0\u67e5\u624b\u673aBluetooth\u662f\u5426\u542f\u7528"), !0), 103 == h.errno ? t.showModalTips(n.$t("\u8bf7Settings\u5c0f\u7a0b\u5e8fBluetooth\u6743\u9650"), !0) : t.showModalTips("Open Bluetooth Adapter Fail"), callbackFunc && callbackFunc(!1)
                                    }
                                })
                            }
                        },
                        setBLEConnectionStateChange: function () {
                            if (!this.BLEConnectionStateChangeSet) {
                                this.BLEConnectionStateChangeSet = !0;
                                var t = this;
                                uni.onBLEConnectionStateChange((function (result) {
                                    t.blu_data_cmdSending = !1,
                                        result.connected || (logger("log", "setBLEConnectionStateChange", t.bleManualDisCnn, " at App.vue:471"),
                                            t.bleManualDisCnn || t.doCloseBLEConnection(result.deviceId),
                                            t.ble_device && t.ble_device.deviceId != result.deviceId || (t.blu_data_canSend = !1,
                                                t.setBluCnnState(0, !0)))
                                }))
                            }
                        },
                    },
                    onLaunch: function () {
                        this.globalData.getDeviceInfo(),
                            this.globalData.getSysinfo();
                    },
                    onHide: function () {
                        this.globalData.closeBLEConnection((function (t) {
                            this.globalData.blu_state = 0,
                                this.globalData.setBluCnnState(0, false),
                                this.globalData.closeBluetoothAdapter()
                        }))
                    }
                };
                t.default = r
            }).call(this, r("enhancedConsoleLogger")["default"])
        },


        "mainPageComponent": function (e, t, r) {
            "use strict";
            (function (logger) {

                var app = r("appStateManager")["default"],
                    deviceCommandUtil = r("deviceCommandUtils "),
                    uni = r("uni"),
                    bleDeviceController = (r("handDrawFileManager"), r("bleDeviceControlUtils ")),
                    module = {
                        data: function () {
                            var deviceFeatures = app.globalData.getDeviceFeatures();
                            return {
                                modeCmdSend: "",
                                functions: [
                                    {
                                        tag: 0,
                                        name: "DMX",
                                        show: !0
                                    },
                                    {
                                        tag: 1,
                                        name: "Random playback",
                                        show: !0
                                    },
                                    {
                                        tag: 2,
                                        name: "Timeline playback",
                                        show: !0
                                    },
                                    {
                                        tag: 3,
                                        name: "Animation playback",
                                        show: !0
                                    },
                                    {
                                        tag: 4,
                                        name: "Text playback",
                                        show: !0
                                    },
                                    {
                                        tag: 5,
                                        name: "Christmas broadcast",
                                        show: !0
                                    },
                                    {
                                        tag: 5,
                                        name: "ILDA",
                                        show: !1
                                    },
                                    {
                                        tag: 6,
                                        name: "Outdoor playback",
                                        show: !0
                                    },
                                    {
                                        tag: 7,
                                        name: "Personalized programming",
                                        show: !0
                                    },
                                    {
                                        tag: 8,
                                        name: "Hand-drawn doodle",
                                        show: !0
                                    },
                                    {
                                        tag: 9,
                                        name: "Playlist",
                                        show: !0
                                    }
                                ],
                                features: deviceFeatures,
                                deviceOn: false,
                                prjIndex: -1,
                                cnnDevice: "Not connected",
                                cnnState: false,
                                randomCheck: [],
                                initShow: false,
                            }
                        },
                        onLoad: function () {
                            this.genRandomCheck();
                        },
                        onShow: function () {
                            this.features = app.globalData.getDeviceFeatures();
                            if (!this.data.initShow) {
                                this.data.initShow = true;
                                this.methods.bluInitPro();
                            }
                        },
                        methods: {
                            bluInitPro: function () {
                                app.globalData.blu_cnn_call_back = this.blu_cnn_call_back,
                                    app.globalData.blu_rec_call_back = this.blu_rec_call_back,
                                    bleDeviceController.cnnPreBlu()
                            },
                            goQueryCmd: function () {
                                bleDeviceController.gosend(!1, deviceCommandUtil.getQueryCmd(this.data.randomCheck));
                            },
                            blu_cnn_call_back: function (connectionStatus, resultCode) {
                                if (1 != connectionStatus) {
                                    var device = app.globalData.ble_device;
                                    logger("log", "blu_cnn_call_back1", connectionStatus, resultCode);
                                    if (connectionStatus && device && "characteristicId" in device)
                                        logger("log", "Connected", device.name),
                                            this.cnnDevice = device.name,
                                            this.cnnState = true,
                                            this.goQueryCmd();
                                    else {
                                        this.cnnState = false,
                                            this.deviceOn = false,
                                            this.prjIndex = -1;
                                    }
                                }
                            },

                            blu_rec_call_back: function (data) {
                                logger("log", "blu_rec_call_back");
                                this.checkRcvData(data, this.randomCheck)
                                    ? (bleDeviceController.setCanSend(true),
                                        bleDeviceController.setCmdData(data),
                                        this.prjIndex = app.globalData.cmd.curMode)
                                    : logger("log", "Abnormality in reading device parameters");
                            },

                            // fill the this.randomCheck array with 4 random integers between 0 and 255 (inclusive). 
                            genRandomCheck: function () {
                                for (var e = 0; e < 4; e++) this.randomCheck[e] = Math.floor(256 * Math.random())
                            },

                            // Validates the received data string and random check array.
                            // Decodes and checks a checksum validation code.
                            // Updates device status and features if valid.
                            // Returns true if the data is valid and processed, otherwise false.
                            checkRcvData: function (data, randomVerify) {
                                if (4 != randomVerify.length || data.length < 24) return !1;
                                for (var r = data.substr(data.length - 24, 8), n = [], h = 0; h < 4; h++) {
                                    var i = 0,
                                        c = randomVerify[h];
                                    0 == h && (i = (c + 55 >> 1) - 10 & 255), 1 == h && (i = 7 + (c - 68 << 1) & 255), 2 == h && (i = 15 + (c + 97 >> 1) & 255), 3 == h && (i = 87 + (c - 127 >> 1) & 255), n.push(i)
                                }
                                for (var o = [], s = 0; s < 4; s++) {
                                    var l = r[2 * s] + r[2 * s + 1],
                                        p = parseInt(l, 16);
                                    o.push(p)
                                }
                                for (var d = 0; d < 4; d++)
                                    if (o[d] != n[d]) return !1;
                                var b = data.substr(data.length - 16, 2);
                                this.deviceOn = 0 != parseInt(b, 16);
                                var deviceType = data.substr(data.length - 14, 2),
                                    version = data.substr(data.length - 12, 2),
                                    userType = data.substr(data.length - 10, 2);
                                return (app.globalData.setDeviceInfo(deviceType, version, userType),
                                    this.features = app.globalData.getDeviceFeatures()),
                                    this.features = app.globalData.getDeviceFeatures(), !0
                            },

                            cnnLaser: function () {
                                bleDeviceController.cnnLaser()
                            },

                            // decides whether to switch to DMX mode and send a command, 
                            // navigate to the settings page, or prompt the user to turn on the device, 
                            // based on the clicked tag and device/debug state.
                            settingClick: function (e) {
                                var tag = e.currentTarget.dataset.tag;
                                if (0 != tag || this.deviceOn)
                                    return this.prjIndex != tag && 0 == tag
                                        ? (this.prjIndex = tag, app.globalData.setCmdMode(tag), void this.sendCmd())
                                        : void uni.navigateTo({ url: "/pages/setting/setting?dmx=" + tag });

                            },

                            // This function handles toggling the device's power state and 
                            // sending the appropriate command, with user feedback if the action cannot be performed.
                            onOffChange: function (t) {
                                if (bleDeviceController.getCanSend()) {
                                    this.deviceOn = !this.deviceOn;
                                    var command = "B0B1B2B300B4B5B6B7";
                                    this.deviceOn && (command = "B0B1B2B3FFB4B5B6B7"), bleDeviceController.gosend(!1, command)
                                } else this.cnnState
                                    ? app.globalData.showModalTips(this.$t("The current device cannot be identified"), !0)
                                    : app.globalData.showModalTips(this.$t("Please connect first 5Bluetooth"), !0)
                            },

                            // testFunc injects a specific test command into the device controller 
                            // and synchronizes the project index with the current command mode, 
                            // probably to simulate or test device behavior with known data.
                            testFunc: function () {
                                bleDeviceController.setCmdData("E0E1E2E3B0B1B2B300B4B5B6B7C0C1C2C30400098080800080003309FFFFFF320000000000000000000000000000000000000000000000000000000000000000000000000000FF035393C06600000000000000000000000000000000000000000000000000000000000000000000000000C4C5C6C7000102030001000A00FFFFFF020000000000000004050607D0D1D2D3820000FF28000000000000000000003200FF00FF28000000000000000000FF3200FFD4D5D6D7F0F1F2F300000000070102030405060700004466F4F5F6F743E3A317F0000000E4E5E6E7"),
                                    this.prjIndex = app.globalData.cmd.curMode
                            },

                            // sendCmd generates a device command string based on the current mode and features, 
                            // stores it, and triggers the sending process.
                            sendCmd: function () {
                                var command = deviceCommandUtil.getCmdStr(app.globalData.cmd, {
                                    features: this.features
                                });
                                this.modeCmdSend = command, this.doSendCmd()
                            },

                            // doSendCmd tries to send the current command. If it fails, it retries every 100ms until successful. 
                            // When successful, it clears the command buffer.
                            doSendCmd: function () {
                                if ("" != this.modeCmdSend) {
                                    var result = bleDeviceController.gosend(!1, this.modeCmdSend),
                                        t = this;
                                    result ? this.modeCmdSend = "" : setTimeout((function () {
                                        t.doSendCmd()
                                    }), 100)
                                }
                            },
                            //  routes the user to the correct function or project page, 
                            // sending the necessary command to the device, 
                            // and handles device state checks and navigation logic.
                            prjClick: function (e) {

                                var mode = e.currentTarget.dataset.tag;

                                if (0 != mode)
                                    if (this.deviceOn) {

                                        if (this.prjIndex != mode || 5 == mode && this.features.ilda) return this.prjIndex = mode,
                                            app.globalData.setCmdMode(mode), void this.sendCmd();

                                        this.sendCmd(),

                                            4 == mode && uni.navigateTo({
                                                url: "/sub/pages/text/text"
                                            }),

                                            7 == mode && uni.navigateTo({
                                                url: "/sub2/pages/pgs/pgs"
                                            }),

                                            8 == mode && uni.navigateTo({
                                                url: "/sub/pages/draw/draw"
                                            }),
                                            9 == mode && uni.navigateTo({
                                                url: "/sub/pages/listMaster/listMaster"
                                            }),

                                            mode >= 1 && mode <= 6 && 4 != mode && uni.navigateTo({
                                                url: "/pages/prj/prj?tag=" + mode
                                            })
                                    }
                                    else this.settingClick(e)
                            },


                            sendLastCmd: function (e) {
                                this.lastCmdTime = (new Date).getTime(), this.sendCmd2(e)
                            },

                        }
                    };
                t.default = module
            }).call(this, r("enhancedConsoleLogger")["default"])
        },


        "DeviceConfigPageController": function (e, t, r) {
            "use strict";
            (function (logger) {
                app = getApp(),
                    deviceCommandUtil = r("deviceCommandUtils "),
                    bleDeviceController = r("bleDeviceControlUtils "),
                    module = (r("geometryAndUuidUtils"), {
                        data: function () {
                            var deviceFeatures = app.globalData.getDeviceFeatures();
                            return {
                                showCtr: {
                                    light1: true,
                                    light2: true,
                                    light3: true,
                                    lightExt: false
                                },
                                deviceInfo: 0,
                                features: deviceFeatures,
                                SetCmdSend: "",
                                version: "",
                                machine: "",
                                dmx: 0,

                                // channel
                                ch: 0,
                                xy: 0,
                                light: 1,
                                cfg: 0,
                                valArr: [1, 10, 10, 10, 10],
                                valRange: [
                                    [1, 512],
                                    [10, 100],
                                    [0, 255],
                                    [0, 255],
                                    [0, 255]
                                ]
                            }
                        },

                        onLoad: function (e) {
                            var t = this;
                            this.version = plus.runtime.version, this.version = this.version.trim();
                            var settingData = app.globalData.getCmdData("settingData");
                            settingData.dmx = e.dmx;
                            var n = this;
                            Object.keys(settingData).forEach((function (e) {
                                n.$set(t, e, settingData[e])
                            })),
                                this.deviceInfo = app.globalData.getDeviceInfo(),
                                this.machine = ("0" + this.deviceInfo.deviceType).slice(-2) + " - " + ("0" + this.deviceInfo.version).slice(-2),
                                this.initData()
                        },

                        onUnload: function () {
                            var settingData = {
                                ch: this.ch, // channel
                                dmx: this.dmx, // 0 or 1
                                xy: this.xy, // 0-7 Normal: X+Y+ X+Y- X-Y- X-Y+ Interchange: X+Y+ X+Y- X-Y- X-Y+
                                light: this.light, // 1 single ,2 dual ,3 full
                                cfg: this.cfg,  //  0 ttl 255 analog
                                lang: this.lang,

                            };
                            app.globalData.setCmdData("settingData", settingData)
                        },

                        methods: {
                            sendCmd: function () {
                                var settingData = {
                                    ch: this.ch,
                                    dmx: this.dmx,
                                    xy: this.xy,
                                    light: this.light,
                                    cfg: this.cfg,
                                    lang: this.lang,
                                    valArr: this.valArr
                                };
                                app.globalData.setCmdData("settingData", settingData);
                                var command = deviceCommandUtil.getSettingCmd(app.globalData.cmd.settingData);
                                this.SetCmdSend = command, this.doSendCmd()
                            },
                            doSendCmd: function () {
                                if ("" != this.SetCmdSend) {
                                    var commandResult = bleDeviceController.gosend(!1, this.SetCmdSend),
                                        t = this;
                                    commandResult ? this.SetCmdSend = "" : setTimeout((function () {
                                        t.doSendCmd()
                                    }), 100)
                                }
                            },
                            initData: function () {
                                if (1 == this.deviceInfo.deviceType) {
                                    this.valRange = [
                                        [1, 512],
                                        [10, 100],
                                        [0, 100],
                                        [0, 100],
                                        [0, 100]
                                    ];
                                    for (var e = 2; e < 5; e++) this.valArr[e] > this.valRange[e][1] && (this.valArr[e] = this.valRange[e][1])
                                } (1 == this.deviceInfo.deviceType
                                    || 0 == this.deviceInfo.deviceType
                                    && this.deviceInfo.version >= 1) && (
                                        this.showCtr = {
                                            light1: false,
                                            light2: false,
                                            light3: false,
                                            lightExt: true
                                        })
                            },



                        }
                    });
                t.default = module
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "bleDeviceProjectConfigPageComponent": function (e, t, r) {
            "use strict";
            (function (e) {

                var app = getApp(),
                    deviceCommandUtils = r("deviceCommandUtils "),
                    deviceBleController = r("bleDeviceControlUtils "),
                    module = {
                        data: function () {
                            var deviceFeatures = app.globalData.getDeviceFeatures();
                            return {

                                prjIndex: 0,
                                sendCmdParmsTimer: null,
                                showOutDoorTips: false,
                                features: deviceFeatures,
                                colorDisplayOrder: [{
                                    name: "Red",
                                    color: "red",
                                    order: 0,
                                    idx: 1
                                }, {
                                    name: "yellow",
                                    color: "yellow",
                                    order: 1,
                                    idx: 4
                                }, {
                                    name: "green",
                                    color: "green",
                                    order: 2,
                                    idx: 2
                                }, {
                                    name: "Cyan",
                                    color: "#00FFFF",
                                    order: 3,
                                    idx: 5
                                }, {
                                    name: "blue",
                                    color: "blue",
                                    order: 4,
                                    idx: 3
                                }, {
                                    name: "purple",
                                    color: "purple",
                                    order: 5,
                                    idx: 6
                                }, {
                                    name: "white",
                                    color: "white",
                                    order: 6,
                                    idx: 7
                                }, {
                                    name: "Jump",
                                    color: "transparent",
                                    order: 7,
                                    idx: 8
                                }, {
                                    name: "RGB",
                                    color: "transparent",
                                    order: 8,
                                    idx: 9
                                }],
                                public: {
                                    txColor: 0, //colorDisplayOrder
                                    rdMode: 0, // Audio trigger mode	0, 255
                                    runSpeed: 10, // 1–100 (typical) Playback speed
                                    soundVal: 20 // Sound sensitivity
                                },
                                item: {
                                    pyMode: 0, // Playback mode
                                    prjSelected: [0, 0, 0, 0], // Pattern selection (bitfield)
                                    ckValues: [] //Checkbox states (UI) 
                                },
                            }
                        },
                        onLoad: function (e) {
                            var prjIndex = e.tag,
                                prjData = app.globalData.getCmdData("prjData"),
                                projectConfig = {},
                                publicSettings = prjData.public;
                            if (1 == prjIndex) projectConfig = {
                                public: publicSettings,
                                prjIndex: prjIndex
                            };
                            else {
                                var index = prjData.prjItem[prjIndex + ""];
                                projectConfig = {
                                    public: publicSettings,
                                    item: index,
                                    prjIndex: prjIndex
                                };
                                var selectionBits = this.getCkValues(projectConfig.item.prjSelected);
                                projectConfig.item["ckValues"] = selectionBits
                            }
                            this.prjIndex = projectConfig.prjIndex,
                                this.public = projectConfig.public,
                                this.item = projectConfig.item,
                                6 == this.prjIndex && this.features.showOutDoorTips && (this.showOutDoorTips = !0)
                        },


                        methods: {
                            sendCmd: function () {
                                var commandParams = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                                this.refreshShow(),
                                    app.globalData.setCmdData("prjData", {
                                        prjIndex: this.prjIndex,
                                        public: this.public,
                                        item: this.item
                                    }),
                                    null == commandParams && (commandParams = {}),
                                    commandParams["features"] = app.globalData.getDeviceFeatures();
                                var command = deviceCommandUtils.getCmdStr(app.globalData.cmd, commandParams),
                                    r = deviceBleController.gosend(!1, command);
                                return r
                            },


                            // This function takes an array of numbers, where each number is treated as a 16-bit value. 
                            // It extracts each bit (from least significant to most significant) from every number 
                            // and flattens all bits into a single array of 0s and 1s. 
                            // This is useful for converting compact bitfield representations into a simple array of booleans for UI display or logic.
                            getCkValues: function (e) {
                                for (var t = [], r = 0; r < e.length; r++)
                                    for (var n = e[r], h = 0; h < 16; h++) {
                                        var a = n >> h & 1;
                                        t.push(a)
                                    }
                                return t
                            },
                            //This function performs the inverse operation. 
                            // It takes an array of bits (0s and 1s) and packs them into an array of four 16-bit integers. 
                            // For every group of 16 bits, it calculates the corresponding integer 
                            // by setting the appropriate bits, then stores it in the result array. 
                            // This is useful for compressing a long list of boolean flags into a smaller, 
                            // more efficient format for storage or transmission.
                            getprjSelected: function (e) {
                                for (var t = 0, r = [0, 0, 0, 0], n = 0; n < e.length; n++) {
                                    var h = n % 16;
                                    1 == e[n] && (t += Math.pow(2, h)), 15 == h && (r[(n + 1) / 16 - 1] = t, t = 0)
                                }
                                return r
                            },
                            // This function handles clicks on "auto" selection buttons. Depending on the value of t, 
                            // it toggles all bits in ckValues 
                            // (if t == 2, it inverts each bit; if t == 3, it clears all bits; otherwise, it sets all bits to 1).
                            //  After updating the bit array, it uses getprjSelected to pack the bits into integers, 
                            // updates the relevant properties using Vue's $set for reactivity, and sends the updated command.
                            selectAutoBtnClick: function (selectionAction) {
                                for (var r = selectionAction, selectionBits = this.item.ckValues, h = 0; h < selectionBits.length; h++) 2 == r ? 1 == selectionBits[h] ? selectionBits[h] = 0 : selectionBits[h] = 1 : selectionBits[h] = 3 == r ? 0 : 1;
                                var packedBits = this.getprjSelected(selectionBits);
                                this.item.prjSelected = packedBits,
                                    this.item.ckValues = selectionBits,
                                    this.sendCmd()
                            },
                            // This function responds to checkbox group changes. 
                            // It receives the indices of checked boxes, 
                            // then sets the corresponding bits in a four-element integer array (r). 
                            // Each checked index is mapped to a specific bit in the appropriate integer.
                            //  The packed result is stored in prjSelected, and the updated state is sent.
                            checkboxChange: function (indices) {
                                var packedBits = [0, 0, 0, 0];
                                for (var n = indices.detail.value, h = 0; h < n.length; h++) {
                                    var a = n[h] - 1,
                                        i = Math.floor(a / 16),
                                        c = a % 16,
                                        o = 1 << c;
                                    packedBits[i] = packedBits[i] | o
                                }
                                this.item.prjSelected = packedBits,
                                    this.sendCmd()
                            },

                            btnSelectClick: function (selectedIndex) {
                                if (0 != this.item.pyMode) {
                                    var selectionBits = this.item.ckValues,
                                        commandParams = null;
                                    1 == selectionBits[selectedIndex]
                                        ? selectionBits[selectedIndex] = 0
                                        : (selectionBits[selectedIndex] = 1, commandParams = {
                                            prjParm: {
                                                prjIndex: this.prjIndex,
                                                selIndex: selectedIndex + 1
                                            }
                                        });
                                    var packedBits = this.getprjSelected(selectionBits);
                                    this.item.prjSelected = packedBits,
                                        this.item.ckValues = selectionBits,
                                        this.sendCmdParms(commandParams)
                                }
                            },

                            sendCmdParms: function (commandParams) {
                                this.sendCmd(commandParams);
                            }

                        }
                    };
                t.default = module
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "deviceCommandUtils ": function (e, t, r) {
            (function (t) {
                var arrayConverionUtil = r("arrayConversionHelper");

                function createIteratorHelper(e, t) {
                    var r = "undefined" !== typeof Symbol && e[Symbol.iterator] || e["@@iterator"];
                    if (!r) {
                        if (Array.isArray(e) || (r = function (e, t) {
                            if (!e) return;
                            if ("string" === typeof e) return arrayLikeToArray(e, t);
                            var r = Object.prototype.toString.call(e).slice(8, -1);
                            "Object" === r && e.constructor && (r = e.constructor.name);
                            if ("Map" === r || "Set" === r) return Array.from(e);
                            if ("Arguments" === r || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(r)) return arrayLikeToArray(e, t)
                        }(e)) || t && e && "number" === typeof e.length) {
                            r && (e = r);
                            var n = 0,
                                h = function () { };
                            return {
                                s: h,
                                n: function () {
                                    return n >= e.length ? {
                                        done: !0
                                    } : {
                                        done: !1,
                                        value: e[n++]
                                    }
                                },
                                e: function (e) {
                                    throw e
                                },
                                f: h
                            }
                        }
                        throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.")
                    }
                    var i, c = !0,
                        o = !1;
                    return {
                        s: function () {
                            r = r.call(e)
                        },
                        n: function () {
                            var e = r.next();
                            return c = e.done, e
                        },
                        e: function (e) {
                            o = !0, i = e
                        },
                        f: function () {
                            try {
                                c || null == r.return || r.return()
                            } finally {
                                if (o) throw i
                            }
                        }
                    }
                }

                function arrayLikeToArray(e, t) {
                    (null == t || t > e.length) && (t = e.length);
                    for (var r = 0, n = new Array(t); r < t; r++) n[r] = e[r];
                    return n
                }

                function toFixedWidthHex(value) {
                    var width = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 4,
                        roundedValue = Math.round(value);
                    roundedValue < 0 && (roundedValue = 32768 | -roundedValue);
                    var hexStringResult = ("0000" + roundedValue.toString(16)).slice(-width);
                    return hexStringResult
                }

                function combineNibbles(e, t) {
                    var r = e << 4 | 15 & t;
                    return r
                }

                function splitIntoSegmentsBySumLimit(numbers, limit) {
                    // console.log("splitIntoSegmentsBySumLimit called with:", numbers, "limit:", limit);
                    let currentSum = 0;
                    let segmentRanges = [];
                    let segmentStartIdx = 0;
                    let segmentSize = 0;
                    for (let numberIdx = 0; numberIdx < numbers.length; numberIdx++) {
                        if (currentSum + numbers[numberIdx] <= limit) {
                            segmentSize += 1;
                            segmentRanges.push([segmentStartIdx, segmentSize]);
                            currentSum += numbers[numberIdx];
                        } else {
                            let tempSum = currentSum;
                            while (true) {
                                if (tempSum <= limit) {
                                    segmentSize += 1;
                                    segmentRanges.push([segmentStartIdx, segmentSize]);
                                    currentSum = tempSum + numbers[numberIdx];
                                    break;
                                }
                                if (tempSum > limit && tempSum - numbers[segmentStartIdx] < limit) {
                                    segmentSize += 1;
                                    segmentRanges.push([segmentStartIdx, segmentSize]);
                                    currentSum += numbers[numberIdx];
                                    break;
                                }
                                tempSum -= numbers[segmentStartIdx];
                                currentSum -= numbers[segmentStartIdx];
                                segmentStartIdx += 1;
                                segmentSize -= 1;
                            }
                        }
                    }
                    console.log("[JS] splitIntoSegmentsBySumLimit result:", segmentRanges);
                    return segmentRanges;
                }

                function generateSegmentedLayoutData(segments, scalingFactor) {
                    var mode = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : 0;
                    var lastSegmentIndex = -1;
                    var segmentWidths = [];
                    var segmentHeights = [];
                    var segmentDefaultSize = 200;
                    var totalSegmentWidth = 0;
                    var totalSegmentHeight = 0;
                    for (var segmentIdx = 0; segmentIdx < segments.length; segmentIdx++) {
                        if (lastSegmentIndex != segments[segmentIdx][0]) {
                            lastSegmentIndex = segments[segmentIdx][0];
                            segmentWidths.push(segments[segmentIdx][2] * scalingFactor);
                            totalSegmentWidth += segments[segmentIdx][2];
                            segmentHeights.push(segments[segmentIdx][3] * scalingFactor);
                            totalSegmentHeight += segments[segmentIdx][3];
                            // Detailed debug output for parity comparison
                            console.log(`[JS] Segment ${segmentIdx}: index=${segments[segmentIdx][0]}, width=${segments[segmentIdx][2]} (scaled=${segments[segmentIdx][2] * scalingFactor}), height=${segments[segmentIdx][3]} (scaled=${segments[segmentIdx][3] * scalingFactor})`);
                        }
                    }
                    // Debug: print segmentWidths and segmentHeights
                    console.log("[JS] generateSegmentedLayoutData segmentWidths:", segmentWidths);
                    console.log("[JS] generateSegmentedLayoutData segmentHeights:", segmentHeights);
                    if (127 == mode || 127 == mode) {
                        var verticalOffset = 0;
                        var verticalFillers = [];
                        for (var verticalFillerIdx = 0; verticalFillerIdx < 9; verticalFillerIdx++) {
                            lastSegmentIndex++;
                            var verticalFillerPoints = [{
                                x: 0,
                                y: totalSegmentHeight / 2 + segmentDefaultSize / 2 + verticalOffset,
                                z: 0
                            }];
                            verticalFillers.push([lastSegmentIndex, verticalFillerPoints, segmentDefaultSize, segmentDefaultSize]);
                            verticalOffset += segmentDefaultSize;
                            segmentHeights.push(segmentDefaultSize * scalingFactor);
                        }
                        var splitVerticalSegments = splitIntoSegmentsBySumLimit(segmentHeights, 800);

                        var verticalStartHex = "";
                        var verticalCountHex = "";
                        for (var splitIdx = 0; splitIdx < splitVerticalSegments.length; splitIdx++) {
                            verticalStartHex += toFixedWidthHex(splitVerticalSegments[splitIdx][0], 2);
                            verticalCountHex += toFixedWidthHex(splitVerticalSegments[splitIdx][1], 2);
                        }
                        return [segments.concat(verticalFillers), verticalStartHex, verticalCountHex, -verticalOffset * scalingFactor / 2];
                    }
                    var horizontalOffset = 0;
                    var horizontalFillers = [];
                    for (var horizontalFillerIdx = 0; horizontalFillerIdx < 9; horizontalFillerIdx++) {
                        lastSegmentIndex++;
                        var horizontalFillerPoints = [{
                            x: totalSegmentWidth / 2 + segmentDefaultSize / 2 + horizontalOffset,
                            y: 0,
                            z: 0
                        }];
                        horizontalFillers.push([lastSegmentIndex, horizontalFillerPoints, segmentDefaultSize, segmentDefaultSize]);
                        horizontalOffset += segmentDefaultSize;
                        segmentWidths.push(segmentDefaultSize * scalingFactor);
                    }
                    var splitHorizontalSegments = splitIntoSegmentsBySumLimit(segmentWidths, 800);
                    var segmentStartHex = "";
                    var segmentCountHex = "";
                    for (var splitIdx = 0; splitIdx < splitHorizontalSegments.length; splitIdx++) {
                        segmentStartHex += toFixedWidthHex(splitHorizontalSegments[splitIdx][0], 2);
                        segmentCountHex += toFixedWidthHex(splitHorizontalSegments[splitIdx][1], 2);
                    }
                    console.log("[JS] generateSegmentedLayoutData :" + segments.concat(horizontalFillers), segmentStartHex, segmentCountHex, -horizontalOffset * scalingFactor / 2);
                    return [segments.concat(horizontalFillers), segmentStartHex, segmentCountHex, -horizontalOffset * scalingFactor / 2];
                }

                function encodeLayoutToCommandData(polylineSegments, segmentTime, commandOptions, mirrorMode) {
                    //   console.log("encodeLayoutToCommandData called with:", polylineSegments, segmentTime, commandOptions, mirrorMode, arguments.length > 4 ? arguments[4] : 0);
                    var a = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : 0;
                    if (0 == polylineSegments.length) return null;
                    var counter = 0,
                        counter2 = 0,
                        prevIndex = -1,
                        command = "",
                        b = "",
                        ver = toFixedWidthHex(a, 2),
                        charPointCmd = "",
                        charWidthCmd = "",
                        V = 8,
                        scalingFactor = .5,
                        F = V,
                        segmentPointCount = 0,
                        time = "00";
                    time = commandOptions.textDecimalTime
                        ? toFixedWidthHex(Math.floor(10 * segmentTime), 2)
                        : toFixedWidthHex(Math.floor(segmentTime), 2),
                        //console.log("encodeLayoutToCommandData time:", time), 
                        V >= 8 && (F = 0);
                    var test = !1;
                    if (test) t("error", "20241210 - Current code is in coordinate adjustment mode and cannot be published.", " at utils/funcTools.js:345"),
                        xyss = polylineSegments, se1 = 0, se2 = 0, xOffset = 0;
                    else {
                        var segementData = generateSegmentedLayoutData(polylineSegments, scalingFactor, mirrorMode);
                        xyss = segementData[0], se1 = segementData[1], se2 = segementData[2], xOffset = segementData[3]
                        // Debug: print segment grouping and metadata
                        console.log("[JS] generateSegmentedLayoutData output:");
                        console.log("  xyss.length:", xyss.length);
                        console.log("  se1:", se1);
                        console.log("  se2:", se2);
                        console.log("  xOffset:", xOffset);
                        // Print segment indices and point counts
                        let segmentBoundaries = [];
                        let segmentPointCounts = [];
                        let lastIndex = null;
                        let currentCount = 0;
                        for (let i = 0; i < xyss.length; i++) {
                            if (xyss[i][0] !== lastIndex) {
                                if (lastIndex !== null) segmentPointCounts.push(currentCount);
                                segmentBoundaries.push(xyss[i][0]);
                                lastIndex = xyss[i][0];
                                currentCount = 0;
                            }
                            currentCount += xyss[i][1].length;
                        }
                        if (lastIndex !== null) segmentPointCounts.push(currentCount);
                        console.log("  segmentBoundaries:", segmentBoundaries);
                        console.log("  segmentPointCounts:", segmentPointCounts);
                    }
                    for (var ix = 0; ix < xyss.length; ix++) {
                        prevIndex != xyss[ix][0] && (prevIndex = xyss[ix][0], counter2 > 0
                            && (charPointCmd += toFixedWidthHex(segmentPointCount, 2),
                                console.log(`[JS] charPointCmd append: seg ${counter2 - 1} count ${segmentPointCount} -> ${toFixedWidthHex(segmentPointCount, 2)}`), segmentPointCount = 0),
                            counter2++,
                            charWidthCmd += toFixedWidthHex(Math.round(Number(xyss[ix][2] * scalingFactor)), 2),
                            console.log(`[JS] charWidthCmd append: seg ${counter2 - 1} width ${Math.round(Number(xyss[ix][2] * scalingFactor))} -> ${toFixedWidthHex(Math.round(Number(xyss[ix][2] * scalingFactor)), 2)}`),
                            V >= 8 && xyss[ix][1].length > 1 && F++),
                            F >= 8 && (F = 1);
                        var segmentPoints = xyss[ix][1];
                        segmentPointCount += segmentPoints.length;
                        for (var index = 0; index < segmentPoints.length; index++) {
                            counter++;
                            var point = segmentPoints[index],
                                xScreen = Math.round(Number(point.x * scalingFactor) + xOffset),
                                yScreen = Math.round(Number(point.y * scalingFactor)),
                                pointType = Number(point.z),
                                segmentIndex = F;
                            0 == index && (segmentIndex = 0, pointType = 1), index == segmentPoints.length - 1
                                && (pointType = 1), 1 == segmentPoints.length && (pointType = Number(point.z)),
                                commandOptions.textStopTime && segmentPoints.length > 1
                                && (0 == segmentIndex ? pointType = 2 : (index < segmentPoints.length - 1
                                    && 0 == segmentPoints[index + 1].s || index == segmentPoints.length - 1)
                                    && (pointType = 3)),
                                command = command + toFixedWidthHex(xScreen) + toFixedWidthHex(yScreen) + toFixedWidthHex(combineNibbles(segmentIndex, pointType), 2);
                            // Debug: print packed point
                            if (ix === 0 && index < 4) {
                                console.log(`[JS] Packed point ${index}: x=${xScreen} y=${yScreen} segIdx=${segmentIndex} type=${pointType} -> ${toFixedWidthHex(xScreen)}${toFixedWidthHex(yScreen)}${toFixedWidthHex(combineNibbles(segmentIndex, pointType), 2)}`);
                            }
                            test && (b = b + "\n{" + xScreen + "," + yScreen + "," + segmentIndex + "," + pointType + "},")
                        }
                    }
                    charPointCmd += toFixedWidthHex(segmentPointCount, 2);
                    console.log(`[JS] charPointCmd final append: seg ${counter2 - 1} count ${segmentPointCount} -> ${toFixedWidthHex(segmentPointCount, 2)}`);
                    // Print all packed fields for parity analysis
                    console.log("[JS] encodeLayoutToCommandData packed fields:");
                    console.log("  cnt:", counter);
                    console.log("  charCount:", counter2);
                    console.log("  cmd:", command);
                    console.log("  charWidthCmd:", charWidthCmd);
                    console.log("  charPointCmd:", charPointCmd);
                    console.log("  se1:", se1);
                    console.log("  se2:", se2);
                    console.log("  ver:", ver);
                    console.log("  time:", time);
                    return test && t("log", "Text coordinates (drawing software format)", b, " at utils/funcTools.js:408"), 0 == counter
                        ? null : {
                            cnt: counter,
                            charCount: counter2,
                            cmd: command,
                            charWidthCmd: charWidthCmd,
                            charPointCmd: charPointCmd,
                            se1: se1,
                            se2: se2,
                            ver: ver,
                            time: time
                        }
                }

                function padHexStringToByteLength(e, t) {
                    for (var r = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : "00", n = Math.floor(e.length / 2), h = e, a = n; a < t; a++) h += r;
                    return h
                }

                function getBitmaskAndIndex(e) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : -1,
                        r = e - 1;
                    t > -1 && (r = t - e);
                    var n = Math.trunc(r / 16),
                        h = r % 16,
                        a = Math.pow(2, h),
                        i = 0;
                    return i = t > -1 ? 65535 & a : 65535 & ~a, {
                        idx: n,
                        val: i,
                        decBy: t
                    }
                }

                function applyBitmaskUpdates(e, t) {
                    var r, a = arrayConverionUtil(t),
                        i = createIteratorHelper(e);
                    try {
                        for (i.s(); !(r = i.n()).done;) {
                            var c = r.value,
                                o = getBitmaskAndIndex(c, -1);
                            if (o.idx < a.length) {
                                var s = a[o.idx] & o.val;
                                if (a[o.idx] != s) {
                                    a[o.idx] = s;
                                    var l = getBitmaskAndIndex(c, 50);
                                    l.idx < a.length && (a[l.idx] = a[l.idx] | l.val)
                                }
                            }
                        }
                    } catch (p) {
                        i.e(p)
                    } finally {
                        i.f()
                    }
                    return a
                }

                function getFeatureValue(e, t) {
                    if (e.hasOwnProperty("features")) {
                        var r = e.features;
                        if (r.hasOwnProperty(t)) return r[t]
                    }
                    return null
                }

                function encodeDrawPointCommand(points, config, features, pointTimeValue) {
                    var headerSuffix = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : "00";
                    var encodedPoints = "";
                    var configHeader = "";
                    for (var configIndex = 0; configIndex < 15; configIndex++) {
                        if (configIndex <= 11) {
                            configHeader += toFixedWidthHex(config.cnfValus[configIndex], 2);
                        } else if (configIndex === 13) {
                            if (getFeatureValue({ features: features }, "picsPlay")) {
                                configHeader += toFixedWidthHex(-1 == pointTimeValue ? 10 * config.cnfValus[12] : 10 * pointTimeValue, 2);
                            } else {
                                configHeader += "00";
                            }
                        } else if (configIndex === 14 && features.textStopTime) {
                            configHeader += toFixedWidthHex(config.txPointTime, 2);
                        } else {
                            configHeader += "00";
                        }
                    }
                    if ("00" == headerSuffix) {
                        configHeader += headerSuffix;
                        for (var pointIndex = 0; pointIndex < points.length; pointIndex++) {
                            var point = points[pointIndex];
                            var pointType = point[3];
                            if (features.textStopTime) {
                                if (point[2] == 0) {
                                    pointType = 2;
                                } else if ((pointIndex < points.length - 1 && points[pointIndex + 1][2] == 0) || pointIndex == points.length - 1) {
                                    pointType = 3;
                                }
                            }
                            encodedPoints
                            += toFixedWidthHex(point[0].toFixed()) 
                            + toFixedWidthHex(point[1].toFixed()) 
                            + toFixedWidthHex(combineNibbles(point[2], pointType), 2);
                        }
                        encodedPoints = configHeader + toFixedWidthHex(points.length) + encodedPoints;
                    } else {
                        configHeader += headerSuffix;
                        encodedPoints = configHeader;
                    }
                    return encodedPoints;
                }

                function drawPointStrToCmd(pointString, config) {
                    var headerSuffix = arguments.length > 2 && void 0 !== arguments[2]
                        ? arguments[2]
                        : null,
                        commandStr = "";
                    return commandStr = null == headerSuffix
                        ? config.picsPlay
                            ? "f0f1f200" + pointString + "f4f5f6f7"
                            : "f0f1f2f3" + pointString + "f4f5f6f7"
                        : "f0f1f2" 
                        + toFixedWidthHex(headerSuffix, 2) 
                        + pointString 
                        + "f4f5f6f7", commandStr.toUpperCase()
                }

                e.exports = {
                    generateSegmentedLayoutData: generateSegmentedLayoutData,
                    encodeLayoutToCommandData: encodeLayoutToCommandData,
                    test: function (e) {
                        return "hello---" + e
                    },
                    inArray: function (e, t, r) {
                        for (var n = 0; n < e.length; n++)
                            if (e[n][t] === r) return n;
                        return -1
                    },
                    ab2hex: function (e) {
                        var t = Array.prototype.map.call(new Uint8Array(e), (function (e) {
                            return ("00" + e.toString(16)).slice(-2) + ""
                        }));
                        return t.join("").toUpperCase()
                    },
                    ab2Str: function (e) {
                        var t = new Uint8Array(e),
                            r = String.fromCharCode.apply(null, t);
                        return r
                    },
                    stringToBytes: function (e) {
                        for (var t, r, n = [], h = 0; h < e.length; h++) {
                            t = e.charCodeAt(h), r = [];
                            do {
                                r.push(255 & t), t >>= 8
                            } while (t);
                            n = n.concat(r.reverse())
                        }
                        return n
                    },
                    getXtsCmd: function (e) {
                        for (var t = e.split("\n"), r = 0, n = "", h = 0; h < t.length; h++) {
                            var a = t[h];
                            if ("" != a) {
                                r++;
                                var i = a.split(","),
                                    c = Number(i[0]) + 400,
                                    o = Number(i[1]) + 400;
                                n = n + ("00" + c.toString(16)).slice(-4) + ("00" + o.toString(16)).slice(-4) + ("00" + Number(i[2]).toString(16)).slice(-2) + ("00" + Number(i[3]).toString(16)).slice(-2)
                            }
                        }
                        return 0 == r ? "" : (n = "55667788" + ("00" + r.toString(16)).slice(-2) + n + "88776655", n)
                    },
                    getXysCmd: function (inputPoints) {
                        var version = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 0;
                        var totalPointCount = 0;
                        var segmentCount = 0;
                        var lastSegmentIndex = -1;
                        var encodedPoints = "";
                        var debugOutput = "";
                        var segmentSizesHex = "";
                        var segmentWidthsHex = "";
                        var versionHex = toFixedWidthHex(version, 2);
                        var segmentWidth = 8;
                        var scalingFactor = 0.5;
                        var segmentIndex = segmentWidth;
                        var currentSegmentSize = 0;
                        if (segmentWidth >= 8) segmentIndex = 0;
                        var segmentedData = generateSegmentedLayoutData(xyss, segmentIndex);
                        xyss = segmentedData[0];
                        var segmentMetadataHex = segmentedData[1] + segmentedData[2];
                        var xOffset = segmentedData[3];
                        for (var segmentIdx = 0; segmentIdx < xyss.length; segmentIdx++) {
                            if (lastSegmentIndex != xyss[segmentIdx][0]) {
                                lastSegmentIndex = xyss[segmentIdx][0];
                                if (segmentCount > 0) segmentSizesHex += toFixedWidthHex(currentSegmentSize, 2);
                                currentSegmentSize = 0;
                                segmentCount++;
                                segmentWidthsHex += toFixedWidthHex(Math.round(Number(xyss[segmentIdx][2] * scalingFactor)), 2);
                                if (segmentWidth >= 8 && xyss[segmentIdx][1].length > 1) segmentIndex++;
                            }
                            if (segmentIndex >= 8) segmentIndex = 1;
                            var segmentPoints = xyss[segmentIdx][1];
                            currentSegmentSize += segmentPoints.length;
                            for (var pointIdx = 0; pointIdx < segmentPoints.length; pointIdx++) {
                                totalPointCount++;
                                var point = segmentPoints[pointIdx];
                                var xScreen = Math.round(Number(point.x * scalingFactor) + xOffset);
                                var yScreen = Math.round(Number(point.y * scalingFactor));
                                var pointType = Number(point.z);
                                var packedSegmentIndex = segmentIndex;
                                if (pointIdx == 0) { packedSegmentIndex = 0; pointType = 1; }
                                if (pointIdx == segmentPoints.length - 1) pointType = 1;
                                if (segmentPoints.length == 1) pointType = Number(point.z);
                                encodedPoints += toFixedWidthHex(xScreen) + toFixedWidthHex(yScreen) + toFixedWidthHex(combineNibbles(packedSegmentIndex, pointType), 2);
                                debugOutput += "\n" + xScreen + "," + yScreen + ",(" + packedSegmentIndex + "," + pointType + "),";
                            }
                        }
                        segmentSizesHex += toFixedWidthHex(currentSegmentSize, 2);
                        if (totalPointCount == 0) return "";
                        encodedPoints = "A0A1A2A3" + toFixedWidthHex(totalPointCount) + toFixedWidthHex(segmentCount, 2) + encodedPoints + segmentWidthsHex + segmentSizesHex + segmentMetadataHex + versionHex + "A4A5A6A7";
                        return encodedPoints.toUpperCase();
                    },
                    getXysCmdArr: function (polylinePoints, commandType, mirrored) {
                        for (var h = arguments.length > 3 && void 0 !== arguments[3]
                            ? arguments[3]
                            : 0, a = [], c = 0; c < polylinePoints.length; c++) {
                            var points = polylinePoints[c].xys,
                                s = mirrored;
                            255 == mirrored && null != polylinePoints[c].XysRight
                                ? points = polylinePoints[c].XysRight
                                : 127 == mirrored && null != polylinePoints[c].XysUp
                                    ? points = polylinePoints[c].XysUp
                                    : 128 == mirrored && null != polylinePoints[c].XysDown
                                        ? points = polylinePoints[c].XysDown
                                        : s = 0;
                            var encodedCOmmandData = encodeLayoutToCommandData(points, polylinePoints[c].time, commandType, s, h);
                            null != encodedCOmmandData && a.push(encodedCOmmandData)
                        }
                        if (0 == a.length) return "";
                        for (var d = 0, b = 0, g = "", j = "", x = "", V = "", f = "", F = "", k = "", m = "", P = 0; P < a.length; P++)
                            d += a[P].cnt, b += a[P].charCount, toFixedWidthHex(a[P].cnt),
                                g += toFixedWidthHex(a[P].charCount, 2),
                                j += a[P].cmd, x += a[P].charWidthCmd,
                                V += a[P].charPointCmd, f += a[P].se1, F += a[P].se2, k += a[P].ver, m += a[P].time;
                        // console.log(d, b);
                        var u = toFixedWidthHex(a.length, 2),
                            X = "A0A1A2A3" + toFixedWidthHex(d) + toFixedWidthHex(b, 2) + j + u + g + x + V + f + F + k + m + "A4A5A6A7";
                        return X.toUpperCase()
                    },


                    getXysCmdSimplified: function (segmentPoints, time) {
                        var versionTag = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : 0;
                        var encodedSegments = [];

                        var encodedCommandData = encodeLayoutToCommandData(
                            segmentPoints,
                            time,
                            0,
                            0,
                            0
                        );
                        if (encodedCommandData != null) encodedSegments.push(encodedCommandData);





                        if (encodedSegments.length == 0) return "";
                        var totalPointCount = 0,
                            totalCharCount = 0,
                            charCountHex = "",
                            commandHex = "",
                            charWidthHex = "",
                            charPointHex = "",
                            se1Hex = "",
                            se2Hex = "",
                            versionHex = "",
                            timeHex = "";
                        for (var segmentIndex = 0; segmentIndex < encodedSegments.length; segmentIndex++) {
                            totalPointCount += encodedSegments[segmentIndex].cnt;
                            totalCharCount += encodedSegments[segmentIndex].charCount;
                            charCountHex += toFixedWidthHex(encodedSegments[segmentIndex].charCount, 2);


                            //console.log(" segment "+segmentIndex + " cmd "+  encodedSegments[segmentIndex].cmd);

                            commandHex += encodedSegments[segmentIndex].cmd;



                            charWidthHex += encodedSegments[segmentIndex].charWidthCmd;
                            charPointHex += encodedSegments[segmentIndex].charPointCmd;
                            se1Hex += encodedSegments[segmentIndex].se1;
                            se2Hex += encodedSegments[segmentIndex].se2;
                            versionHex += encodedSegments[segmentIndex].ver;
                            timeHex += encodedSegments[segmentIndex].time;
                        }
                        console.log(totalPointCount, totalCharCount);
                        var segmentCountHex = toFixedWidthHex(encodedSegments.length, 2);
                        // Log all command parts for inspection
                        console.log("segmentCountHex:", segmentCountHex);
                        console.log("totalPointCount:", toFixedWidthHex(totalPointCount));
                        console.log("totalCharCount:", toFixedWidthHex(totalCharCount, 2));
                        console.log("commandHex:", commandHex);
                        console.log("charCountHex:", charCountHex);
                        console.log("charWidthHex:", charWidthHex);
                        console.log("charPointHex:", charPointHex);
                        console.log("se1Hex:", se1Hex);
                        console.log("se2Hex:", se2Hex);
                        console.log("versionHex:", versionHex);
                        console.log("timeHex:", timeHex);

                        var resultCmd = "A0A1A2A3" +
                            toFixedWidthHex(totalPointCount) +
                            toFixedWidthHex(totalCharCount, 2) +
                            commandHex +
                            segmentCountHex +
                            charCountHex +
                            charWidthHex +
                            charPointHex +
                            se1Hex +
                            se2Hex +
                            versionHex +
                            timeHex +
                            "A4A5A6A7";
                        return resultCmd.toUpperCase();
                    },

                    getCmdStr: function (commandConfig) {
                        var featureParams = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null;
                        var curModeHex = toFixedWidthHex(commandConfig.curMode, 2);
                        var reservedHex = toFixedWidthHex(0, 2);
                        var colorHex = toFixedWidthHex(commandConfig.textData.txColor, 2);
                        var textSizeHex = toFixedWidthHex(commandConfig.textData.txSize / 100 * 255, 2);
                        var textSizeHex2 = toFixedWidthHex(commandConfig.textData.txSize / 100 * 255, 2);
                        var runSpeedHex = toFixedWidthHex(commandConfig.textData.runSpeed / 100 * 255, 2);
                        var reservedZeroHex = "00";
                        var textDistanceHex = toFixedWidthHex(commandConfig.textData.txDist / 100 * 255, 2);
                        var audioTriggerModeHex = toFixedWidthHex(commandConfig.prjData.public.rdMode, 2);
                        var soundSensitivityHex = toFixedWidthHex(commandConfig.prjData.public.soundVal / 100 * 255, 2);
                        var colorGroupHex = "ffffffff0000";

                        if (null != featureParams) {
                            colorGroupHex = "";
                            if (featureParams.hasOwnProperty("groupList")) {
                                for (var groupIdx = 0; groupIdx < featureParams.groupList.length; groupIdx++) {
                                    colorGroupHex += toFixedWidthHex(featureParams.groupList[groupIdx].color, 2);
                                }
                            }
                            colorGroupHex += "ffffffff";
                            colorGroupHex = colorGroupHex.substring(0, 8);
                            if (getFeatureValue(featureParams, "textStopTime")) {
                                colorGroupHex += toFixedWidthHex(commandConfig.textData.txPointTime, 2);
                            }
                            colorGroupHex += "0000";
                            colorGroupHex = colorGroupHex.substring(0, 12);
                        }

                        var playbackHex = "";
                        var projectItems = commandConfig.prjData.prjItem;
                        for (var projectIdx in projectItems) {
                            var projectItem = projectItems[projectIdx];
                            var playbackMode = projectItem.pyMode == 0 ? 0 : 128;
                            if (playbackMode != 0 && featureParams != null && featureParams.hasOwnProperty("prjParm") && featureParams.prjParm.prjIndex == projectIdx) {
                                if (projectIdx == 3 && getFeatureValue(featureParams, "animationFix") 
                                    && [2, 4, 11, 13, 19].includes(featureParams.prjParm.selIndex)) {
                                    playbackMode |= 50 - featureParams.prjParm.selIndex;
                                } else {
                                    playbackMode |= featureParams.prjParm.selIndex;
                                }
                            }
                            var playbackModeHex = toFixedWidthHex(playbackMode, 2);
                            var selectionBitsHex = "";
                            var selectionBits = arrayConverionUtil(projectItem.prjSelected);
                            if (projectIdx == 3 && getFeatureValue(featureParams, "animationFix")) {
                                selectionBits = applyBitmaskUpdates([2, 4, 11, 13, 19], selectionBits);
                            }
                            for (var selectionIdx = 0; selectionIdx < selectionBits.length; selectionIdx++) {
                                selectionBitsHex = toFixedWidthHex(selectionBits[selectionIdx]) + selectionBitsHex;
                            }
                            playbackHex += playbackModeHex + selectionBitsHex;
                        }
                        var runDirectionHex = "";
                        if (getFeatureValue(featureParams, "arbPlay")) {
                            runDirectionHex += toFixedWidthHex(commandConfig.textData.runDir, 2);
                        }
                        var paddingHex = "";
                        var runDirectionLen = Math.floor(runDirectionHex.length / 2);
                        for (var padIdx = runDirectionLen; padIdx < 44; padIdx++) {
                            paddingHex += "00";
                        }
                        var command = "c0c1c2c3" 
                            + curModeHex 
                            + reservedHex 
                            + colorHex 
                            + textSizeHex 
                            + textSizeHex2 
                            + runSpeedHex 
                            + reservedZeroHex 
                            + textDistanceHex 
                            + audioTriggerModeHex 
                            + soundSensitivityHex 
                            + colorGroupHex 
                            + playbackHex 
                            + runDirectionHex 
                            + paddingHex + "c4c5c6c7";
                        return command.toUpperCase();
                    },

                    getShakeCmdStr: function (e) {
                        var r = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null,
                            n = "";
                        if (getFeatureValue(r, "xyCnf")) {
                            n = "00", r.hasOwnProperty("xyCnfSave") && !r.xyCnfSave && (n = "ff");
                            var a = e.subsetData.xyCnf;
                            a.auto ? n += toFixedWidthHex(a.autoValue, 2) : n += toFixedWidthHex(255 - a.autoValue, 2), n += toFixedWidthHex(a.phase, 2);
                            var c, o = createIteratorHelper(a.xy);
                            try {
                                for (o.s(); !(c = o.n()).done;) {
                                    var s = c.value;
                                    n += toFixedWidthHex(s.value, 2)
                                }
                            } catch (d) {
                                o.e(d)
                            } finally {
                                o.f()
                            }
                            t("log", "xyCnf", JSON.stringify(a), " at utils/funcTools.js:551")
                        }
                        n = padHexStringToByteLength(n, 16, "00");
                        var l = "10111213" + n + "14151617";
                        return l.toUpperCase()
                    },
                    getDrawPointStr: encodeDrawPointCommand,
                    getDrawCmdStr: function (drawPoints, drawconfig, features) {
                        var pointTime = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : "00",
                            encodedDrawCmd = encodeDrawPointCommand(drawPoints, drawconfig, features, -1, pointTime);
                        return drawPointStrToCmd(encodedDrawCmd, features)
                    },
                    drawPointStrToCmd: drawPointStrToCmd,
                    getPisCmdStr: function (segmentIndex, segmentConfig) {
                        var featureParams = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : null;
                        var configValues = segmentConfig.cnfValus;
                        var startMarker = "01";
                        var segmentIndexHex = toFixedWidthHex(segmentIndex, 2);
                        var packedHex = startMarker + segmentIndexHex;
                        for (var configIdx = 0; configIdx <= 12; configIdx++) {
                            packedHex += toFixedWidthHex(configValues[configIdx], 2);
                        }
                        var playTimeHex = toFixedWidthHex(10 * r.playTime, 2);
                        packedHex += playTimeHex;
                        if (getFeatureValue(featureParams, "xyCnf")) {
                            for (var extraIdx = 14; extraIdx <= 18; extraIdx++) {
                                packedHex += toFixedWidthHex(configValues[extraIdx], 2);
                            }
                            t("log", "13-17", configValues[14], configValues[15], configValues[16], configValues[17], configValues[18], " at utils/funcTools.js:516");
                            packedHex = padHexStringToByteLength(packedHex, 24, "00");
                        } else {
                            packedHex = padHexStringToByteLength(packedHex, 18, "00");
                        }
                        var fullCommand = "d0d1d2d3" + packedHex + "d4d5d6d7";
                        return fullCommand.toUpperCase();
                    },
                    
                    getPisListCmdStr: function (segmentList) {
                        for (
                            var featureParams = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null,
                            segmentCountHex = toFixedWidthHex(128 | e.length, 2),
                            segmentEndMarker = "FF",
                            encodedSegments = "",
                            segmentIdx = 0;
                            segmentIdx < e.length;
                            segmentIdx++
                        ) {
                            var segmentHex = "";
                            var segmentObj = e[segmentIdx];
                            for (var valueIdx = 0; valueIdx <= 12; valueIdx++) {
                                segmentHex += toFixedWidthHex(segmentObj.cnfValus[valueIdx], 2);
                            }
                            var playTimeHex = toFixedWidthHex(10 * segmentObj.playTime, 2);
                            segmentHex += playTimeHex;
                            if (getFeatureValue(featureParams, "xyCnf")) {
                                var cnfValusArr = segmentObj.cnfValus;
                                for (var extraIdx = 14; extraIdx <= 18; extraIdx++) {
                                    segmentHex += toFixedWidthHex(cnfValusArr[extraIdx], 2);
                                }
                                t("log", "pgs 14-18", cnfValusArr[14], cnfValusArr[15], cnfValusArr[16], cnfValusArr[17], cnfValusArr[18], " at utils/funcTools.js:488");
                                segmentHex = padHexStringToByteLength(segmentHex, 21, "00");
                            } else {
                                segmentHex = padHexStringToByteLength(segmentHex, 15, "00");
                            }
                            encodedSegments += segmentHex + segmentEndMarker;
                        }
                        encodedSegments = "d0d1d2d3" + segmentCountHex + "00" + encodedSegments + "d4d5d6d7";
                        return encodedSegments.toUpperCase();
                    },
                    getSettingCmd: function (settingData) {
                        var channel = toFixedWidthHex(settingData.valArr[0]),
                            channelSetting = toFixedWidthHex(settingData.ch, 2),
                            displayValue = toFixedWidthHex(settingData.valArr[1], 2),
                            xy = toFixedWidthHex(settingData.xy, 2),
                            redValue = toFixedWidthHex(settingData.valArr[2], 2),
                            greenValue = toFixedWidthHex(settingData.valArr[3], 2),
                            blueValue = toFixedWidthHex(settingData.valArr[4], 2),
                            lightMode = toFixedWidthHex(settingData.light, 2),
                            ttlAnalog = toFixedWidthHex(settingData.cfg, 2);
                        0 == settingData.cfg && (redValue = "FF", greenValue = "FF", blueValue = "FF");
                        var lang = toFixedWidthHex(settingData.lang, 2),
                            command = "00010203" + channel + channelSetting + displayValue + xy + redValue + greenValue + blueValue + lightMode + ttlAnalog + lang + "000000000004050607";
                        return command.toUpperCase()
                    },
                    getCmdValue: function (startPattern, endPattern, inputString) {
                        var matcher = new RegExp(startPattern + "(.+?)" + endPattern),
                            matchResult = matcher.exec(inputString);
                        return null !== matchResult ? matchResult[1] : (t("log", "No matching string found that meets the requirements", startPattern, endPattern, " at utils/funcTools.js:7"), "")
                    },
                    getQueryCmd: function (randomData) {
                        for (var encodedRandomBytes = "", i = 0; i < randomData.length; i++) encodedRandomBytes += toFixedWidthHex(randomData[i], 2);
                        var queryCommand = "E0E1E2E3" + encodedRandomBytes + "E4E5E6E7";
                        return queryCommand.toUpperCase()
                    },
                    getDrawLineStr: function (points, config) {
                        let encodedLine = "";
                        for (let pointIdx = 0; pointIdx < points.length; pointIdx++) {
                            let pointObj = points[pointIdx];
                            encodedLine += toFixedWidthHex(pointObj.pt.x);
                            encodedLine += toFixedWidthHex(pointObj.pt.y);
                            encodedLine += toFixedWidthHex(combineNibbles(pointObj.color, pointObj.z), 2);
                        }
                        encodedLine = "10111213" + toFixedWidthHex(config) + toFixedWidthHex(points.length, 2) + encodedLine + "14151617";
                        return encodedLine.toUpperCase();
                    },
                    getFeaturesValue: getFeatureValue
                }
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "bleDeviceControlUtils": function (e, t, r) {
            (function (t) {
                var appStateManager = getApp(),
                    deviceCommandUtils = r("deviceCommandUtils ");


                function discoverAndConfigureCharacteristics(deviceId, serviceId, retryCount) {
                    var callback = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : null;
                    if (appStateManager.globalData.blu_connect_stop) callback && callback(!1);
                    else {
                        var i = !1,
                            c = "",
                            s = -1;
                        uni.getBLEDeviceCharacteristics({
                            deviceId: deviceId,
                            serviceId: serviceId,
                            success: function (h) {
                                s = 0, t("log", "getBLEDeviceCharacteristics success", h.characteristics, " at utils/bluCtrl.js:173");
                                for (var o = function (o) {
                                    var characteristicInfo = h.characteristics[o];
                                    characteristicInfo.properties.read && (c = characteristicInfo.uuid, i && uni.readBLECharacteristicValue({
                                        deviceId: deviceId,
                                        serviceId: serviceId,
                                        characteristicId: characteristicInfo.uuid,
                                        success: function (e) {
                                            t("log", "readBLECharacteristicValue1:", e, " at utils/bluCtrl.js:184")
                                        },
                                        fail: function (e) {
                                            t("log", "readBLECharacteristicValue1-fail:", e, " at utils/bluCtrl.js:187")
                                        }
                                    })), characteristicInfo.properties.write && -1 != appStateManager.globalData.mtxduuids.indexOf(characteristicInfo.uuid) && (appStateManager.globalData.ble_device.characteristicId = characteristicInfo.uuid, appStateManager.globalData.ble_device.serviceId = serviceId, s++), (characteristicInfo.properties.notify || characteristicInfo.properties.indicate) && -1 != appStateManager.globalData.mrxduuids.indexOf(characteristicInfo.uuid) && uni.notifyBLECharacteristicValueChange({
                                        deviceId: deviceId,
                                        serviceId: serviceId,
                                        characteristicId: characteristicInfo.uuid,
                                        state: !0,
                                        success: function (h) {
                                            appStateManager.globalData.blu_readyRec = !0, i = !0, "" != c && uni.readBLECharacteristicValue({
                                                deviceId: deviceId,
                                                serviceId: serviceId,
                                                characteristicId: characteristicInfo.uuid,
                                                success: function (e) {
                                                    t("log", "readBLECharacteristicValue2:", e, " at utils/bluCtrl.js:217")
                                                },
                                                fail: function (e) {
                                                    t("log", "readBLECharacteristicValue2-fail:", e, " at utils/bluCtrl.js:220")
                                                }
                                            }), appStateManager.globalData.setBluCnnState(2, !1), callback && callback(!0)
                                        },
                                        fail: function (e) {
                                            s > 0 && (appStateManager.globalData.blu_readyRec = !0, i = !0, appStateManager.globalData.setBluCnnState(2, !1), callback && callback(!0))
                                        }
                                    })
                                }, l = 0; l < h.characteristics.length; l++) o(l)
                            },
                            fail: function (e) {
                                0 == retryCount && appStateManager.globalData.showModalTips(translate("Connection failed") + "-1002"), s = -2
                            },
                            complete: function () {
                                s <= 0 && (retryCount > 0 ? setTimeout((function () {
                                    discoverAndConfigureCharacteristics(deviceId, serviceId, --retryCount, callback)
                                }), 1500) : (uni.hideLoading(), callback && callback(!1), appStateManager.globalData.showModalTips(translate("Connection failed") + "-1001")))
                            }
                        })
                    }
                }


                function discoverAndSetupServices(e, r) {
                    var h = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : 3,
                        a = r.callback;
                    if (appStateManager.globalData.blu_connect_stop) a && a(!1);
                    else if (h <= 0) a && a(!1);
                    else {
                        appStateManager.globalData.blu_readyRec = !1;
                        var i = e,
                            c = !1;
                        uni.getBLEDeviceServices({
                            deviceId: i,
                            success: function (e) {
                                t("log", "services: ", e, " at utils/bluCtrl.js:301");
                                for (var r = 0; r < e.services.length; r++)
                                    if (-1 != appStateManager.globalData.mserviceuuids.indexOf(e.services[r].uuid)) {
                                        c = !0, setupCharacteristicNotification(i, e.services[r].uuid, a);
                                        break
                                    }
                            },
                            fail: function (e) {
                                t("log", "getBLEDeviceServices fail:", JSON.stringify(e), " at utils/bluCtrl.js:311")
                            },
                            complete: function () {
                                c || setTimeout((function () {
                                    discoverAndSetupServices(i, r, --h)
                                }), 1e3)
                            }
                        })
                    }
                }

                function connectToDevice(device) {
                    var showMsg = arguments.length > 1 && void 0 !== arguments[1] && arguments[1],
                        connectionCallback = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : null;
                    if (appStateManager.globalData.blu_connect_stop) connectionCallback && connectionCallback(!1);
                    else if (void 0 != device && "" != device && null != device) {
                        appStateManager.globalData.readSetting(), appStateManager.globalData.blu_readyRec = !1;
                        var h = device.deviceId;
                        appStateManager.globalData.createBLEConnection(h, (function (e) {
                            e ? (appStateManager.globalData.setBluCnnState(1, !1), discoverAndSetupServices(h, {
                                showMsg: showMsg,
                                callback: connectionCallback
                            })) : (uni.hideLoading(), showMsg && appStateManager.globalData.showModalTips(translate("Connection failed"), !0), connectionCallback && connectionCallback(!1))
                        }))
                    } else connectionCallback && connectionCallback(!1)
                }



                function canSendBleData() {
                    return appStateManager.globalData.blu_data_canSend
                }

                function logHexBytes(byteArray) {
                    for (var r = "", n = 0; n < byteArray.length; n++) n % 2 == 0 ? ("" != r && (r += ", "), r = r + "0x" + byteArray[n]) : r += byteArray[n];
                    t("log", r, " at utils/bluCtrl.js:494")
                }

                function extractHexValue(startByte, byteLength, hexString) {
                    var n = 2 * (startByte - 1),
                        h = n + 2 * byteLength,
                        a = hexString.slice(n, h),
                        i = parseInt(a, 16);
                    return i
                }

                function clampOrDefault(value, min, max, defaultValue) {
                    return isNaN(value) || value < min || value > max ? defaultValue : value
                }

                function setupCharacteristicNotification(deviceId, serviceId) {
                    var callback = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : null;
                    appStateManager.globalData.blu_connect_stop ? callback && callback(!1)
                        : (uni.onBLECharacteristicValueChange((function (characteristicEvent) {
                            var dataBytes = new Uint8Array(characteristicEvent.value),
                                valueHex = deviceCommandUtils.ab2hex(characteristicEvent.value);
                            deviceCommandUtils.ab2Str(characteristicEvent.value); - 1 != appStateManager.globalData.mrxduuids.indexOf(characteristicEvent.characteristicId) ? appStateManager.globalData.blu_readyRec && dataBytes.length > 0 && processReceivedDataFragment(valueHex) : t("error", "no same characteristicId: ", appStateManager.globalData.mrxduuids, characteristicEvent.characteristicId, " at utils/bluCtrl.js:270")
                        })), discoverAndConfigureCharacteristics(deviceId, serviceId, 1, callback))
                }


                // The processReceivedDataFragment function is designed to handle incoming fragments of data, 
                // likely from a Bluetooth device, and assemble them into complete messages based on specific
                //  start and end markers. It works by maintaining a buffer, blu_rec_content, in the global 
                // application state (appStateManager.globalData). When a new fragment (dataFragment) arrives,
                //  the function checks if the buffer is null. If so, it only initializes the buffer if the 
                // fragment starts with the expected start marker "E0E1E2E3". Otherwise, it appends the new 
                // fragment to the existing buffer.

                // Once the buffer is updated, the function checks if it is non-empty. It then searches for 
                // the last occurrence of the start marker ("E0E1E2E3") and the end marker ("E4E5E6E7") w
                // ithin the buffer. If the end marker is found at the very end of the buffer, 
                // it extracts the complete message from the start marker to the end marker, 
                // calls a callback (setRecCallBack) to process the complete message, 
                // and clears the buffer. If the end marker is found elsewhere, 
                // it keeps only the data from the last start marker onward, 
                // likely waiting for more data to complete the message. Finally,
                //  it updates the buffer in the global state with the remaining or new data.
                function processReceivedDataFragment(dataFragment) {
                    var receiveBuffer = appStateManager.globalData.blu_rec_content;
                    if (null == receiveBuffer ? dataFragment.startsWith("E0E1E2E3") && (receiveBuffer = dataFragment) : receiveBuffer += dataFragment, "" != receiveBuffer) {
                        var r = receiveBuffer.lastIndexOf("E0E1E2E3"),
                            h = receiveBuffer.lastIndexOf("E4E5E6E7"),
                            currentMessage = receiveBuffer;
                        h > 0 && (h == receiveBuffer.length - 8 ? (currentMessage = receiveBuffer.slice(r, h + 8), appStateManager.globalData.setRecCallBack(currentMessage), currentMessage = null) : currentMessage = receiveBuffer.slice(r)), appStateManager.globalData.blu_rec_content = currentMessage
                    }
                }


                function sendBleDataBuffers(sendContext) {
                    var lastSendTimestamp = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 0,
                        sendInterval = 20,
                        a = appStateManager.globalData.blu_data_send_interval;
                    if (appStateManager.globalData.platform.app && "android" == appStateManager.globalData.platform.system && (sendInterval = 40), sendContext.showMsg) {
                        sendContext.count;
                        var i = Math.floor((sendContext.count - sendContext.sendBufs.length) / sendContext.count * 100),
                            c = (new Date).getTime();
                        (100 == i || c - appStateManager.globalData.blu_data_lastShowTime > 200) && (appStateManager.globalData.blu_data_lastShowTime = c, sendContext.callBack ? (uni.hideLoading(), sendContext.callBack(0, i)) : uni.showLoading({
                            mask: !0
                        }))
                    }
                    if (0 != sendContext.sendBufs.length) {
                        var nowTimestamp = (new Date).getTime();
                        lastSendTimestamp = 0 == lastSendTimestamp ? nowTimestamp : lastSendTimestamp;
                        var timeUntilNextSend = sendInterval - (nowTimestamp - lastSendTimestamp),
                            delayBeforeSend = timeUntilNextSend > 0 ? timeUntilNextSend : 1;
                        setTimeout((function () {
                            var currentBuffer = sendContext.sendBufs.shift();
                            "split" != currentBuffer ? (t("log", "send date---", (new Date).getTime() / 1e3, " at utils/bluCtrl.js:441"), uni.writeBLECharacteristicValue({
                                deviceId: sendContext.device.deviceId,
                                serviceId: sendContext.device.serviceId,
                                characteristicId: sendContext.device.characteristicId,
                                value: currentBuffer,
                                success: function (t) {
                                    sendBleDataBuffers(sendContext, nowTimestamp)
                                },
                                fail: function (r) {
                                    t("log", "writeBLECharacteristicValue fail", r, " at utils/bluCtrl.js:454"), setTimeout((function () {
                                        sendContext.fail(r)
                                    }), sendInterval)
                                },
                                complete: function (e) { }
                            })) : setTimeout((function () {
                                t("log", "sleep---", a, sendInterval, " at utils/bluCtrl.js:436"), sendBleDataBuffers(sendContext, nowTimestamp)
                            }), a - (nowTimestamp - lastSendTimestamp))
                        }), delayBeforeSend)
                    } else setTimeout((function () {
                        sendContext.success({})
                    }), sendInterval)
                }


                function splitHexStringToBuffers(hexString) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 20;
                    if ("" == hexString) return [];
                    var r = new Uint8Array(hexString.match(/[\da-f]{2}/gi).map((function (e) {
                        return parseInt(e, 16)
                    })));
                    if (null == r) return [];
                    var n = r.buffer.byteLength,
                        h = 0,
                        a = [];
                    while (n > 0) {
                        var i = n % t,
                            c = void 0;
                        n >= t ? (c = new Uint8Array(r.subarray(h, h + t)).buffer, n -= t, h += t) : (c = new Uint8Array(r.subarray(h, h + i)).buffer, n -= i, h += i), a.push(c)
                    }
                    return a
                }

                function hexStringToBufferSequence(hexString) {
                    var hexSegments = hexString.toUpperCase().split("Z");
                    t("log", hexSegments, " at utils/bluCtrl.js:530");
                    for (var bufferList = [], h = 0; h < hexSegments.length; h++) {
                        t("log", h, hexSegments[h], " at utils/bluCtrl.js:533");
                        var a = splitHexStringToBuffers(hexSegments[h]);
                        a.length > 0 && (bufferList.length > 0 && bufferList.push("split"), bufferList = bufferList.concat(a))
                    }
                    return bufferList
                }

                function sendBleBuffersPromise(dataBuffers, deviceInfo, showProgress) {
                    var progressCallback = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : null;
                    return new Promise((function (h, a) {
                        sendBleDataBuffers({
                            device: deviceInfo,
                            sendBufs: dataBuffers,
                            count: dataBuffers.length,
                            showMsg: showProgress,
                            callBack: progressCallback,
                            success: function (e) {
                                h(e)
                            },
                            fail: function (e) {
                                a(e)
                            }
                        })
                    }))
                }




                e.exports = {

                    // manages the process of sending data over Bluetooth Low Energy (BLE) and handles various edge 
                    // cases and UI feedback.
                    // In summary, gosend returns true if the send process (real or simulated) is started or handled, 
                    // and false if it is blocked due to an ongoing send or invalid data. The actual BLE send is asynchronous,
                    //  so the return value does not indicate send success, only that the process was started or handled.
                    gosend: function (showProgress, hexData) {
                        var sendCallback = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : null;
                        if (canSendBleData()
                            ? logHexBytes(hexData)
                            : 0 == hexData.length || !canSendBleData()
                            && !hexData.startsWith("E0E1E2E3"))
                            return 0 == hexData.length || (t("log", "Simulate sending ------- 20ms",
                                appStateManager.globalData.blu_data_cmdSending,
                                " at utils/bluCtrl.js:552"),
                                !appStateManager.globalData.blu_data_cmdSending
                                && (appStateManager.globalData.blu_data_cmdSending = !0,

                                    setTimeout((function () {
                                        appStateManager.globalData.blu_data_cmdSending = !1,
                                            sendCallback && sendCallback(1, 100)
                                    }), 20), !0));
                        if (appStateManager.globalData.blu_data_cmdSending)
                            return t("error", "last cmd is sending", " at utils/bluCtrl.js:563"), !1;
                        if (2 != appStateManager.globalData.blu_connected)
                            return appStateManager.globalData.showModalTips(translate("Bluetooth not connected")), !0;
                        showProgress && (appStateManager.globalData.blu_data_lastShowTime = (new Date).getTime(),
                            sendCallback ? sendCallback(0, 0) : uni.showLoading({
                                mask: !0
                            }));
                        var bufferSequence = hexStringToBufferSequence(hexData);
                        if (0 == bufferSequence.length) return !1;
                        if (appStateManager.globalData.blu_data_cmdSending) return !1;
                        appStateManager.globalData.blu_data_cmdSending = !0;
                        var i = bufferSequence,
                            c = appStateManager.globalData.ble_device;
                        return sendBleBuffersPromise(i, c, showProgress, sendCallback).then((function (r) {
                            showProgress && uni.hideLoading(), appStateManager.globalData.blu_data_cmdSending = !1, t("log", "bluSend succ", " at utils/bluCtrl.js:592"), sendCallback && sendCallback(1, 100)
                        })).catch((function (r) {
                            showProgress && uni.hideLoading(), t("log", "Sending failed", r, " at utils/bluCtrl.js:596"), appStateManager.globalData.blu_data_cmdSending = !1, sendCallback && sendCallback(-1, 0)
                        })), !0
                    },

                    // Initiates BLE connection if not already connected
                    cnnPreBlu: function () {
                        if (0 == appStateManager.globalData.blu_state) {
                            appStateManager.globalData.blu_state = 1,
                                appStateManager.globalData.blu_connect_stop = !1,
                                appStateManager.globalData.readDevice();
                            var e = appStateManager.globalData.ble_device;
                            void 0 != e && "" != e && null != e ? appStateManager.globalData.openBluetoothAdapter((function (r) {
                                r && connectToDevice(e, !1, (function (e) {
                                    1 == appStateManager.globalData.blu_state && (appStateManager.globalData.blu_state = 0)

                                }))
                            })) : appStateManager.globalData.blu_state = 0
                        }
                    },

                    setCanSend: function (canSend) {
                        appStateManager.globalData.blu_data_canSend = canSend
                    },
                    getCanSend: canSendBleData,

                    drawProgress: function (canvas, size, progress) {
                        canvas.beginPath(), canvas.setFillStyle("#4C4C4C");
                        var n = size - 0,
                            h = n;
                        canvas.moveTo(20, 0), canvas.lineTo(0 + n - 20, 0), canvas.arcTo(0 + n, 0, 0 + n, 20, 20), canvas.lineTo(0 + n, 0 + h - 20), canvas.arcTo(0 + n, 0 + h, 0 + n - 20, 0 + h, 20), canvas.lineTo(20, 0 + h), canvas.arcTo(0, 0 + h, 0, 0 + h - 20, 20), canvas.lineTo(0, 20), canvas.arcTo(0, 0, 20, 0, 20), canvas.fill();
                        var a = size / 2,
                            i = a,
                            c = size / 3,
                            o = -Math.PI / 2,
                            s = 2 * Math.PI * progress / 100 + o;
                        canvas.setLineWidth(size / 30), canvas.beginPath(), canvas.arc(a, i, c, 0, 2 * Math.PI), canvas.setStrokeStyle("#616161"), canvas.stroke(), canvas.beginPath(), canvas.arc(a, i, c, o, s), canvas.setStrokeStyle("#ECECEC"), canvas.stroke(), canvas.beginPath();
                        var l = progress + "%",
                            p = size / 5;
                        canvas.setFillStyle("#ECECEC"), canvas.setFontSize(p);
                        var d = canvas.measureText(l).width;
                        canvas.fillText(progress + "%", a - d / 2, i + p / 3), canvas.fill(), canvas.draw()
                    },

                    // responsible for parsing and updating the application's global state based on a device's response data,
                    // typically received as a buffer or hex string. It performs several key tasks:
                    setCmdData: function (deviceResponseData) {

                        deviceCommandUtils.getCmdValue("B0B1B2B3", "B4B5B6B7", deviceResponseData);

                        var mainCommandData = deviceCommandUtils.getCmdValue("C0C1C2C3", "C4C5C6C7", deviceResponseData);

                        appStateManager.globalData.cmd.curMode = clampOrDefault(extractHexValue(1, 1, mainCommandData), 0, 12, 0),

                            appStateManager.globalData.cmd.prjData.prjIndex = clampOrDefault(extractHexValue(1, 1, mainCommandData), 0, 12, 0),

                            appStateManager.globalData.cmd.prjData.public.txColor = clampOrDefault(extractHexValue(3, 1, mainCommandData), 0, 9, 0),

                            appStateManager.globalData.cmd.textData.txColor = appStateManager.globalData.cmd.prjData.public.txColor,
                            appStateManager.globalData.cmd.textData.txSize = clampOrDefault(Math.round(extractHexValue(4, 1, mainCommandData) / 255 * 100), 10, 100, 60),
                            appStateManager.globalData.cmd.textData.runSpeed = clampOrDefault(Math.round(extractHexValue(6, 1, mainCommandData) / 255 * 100), 0, 255, 128),

                            appStateManager.globalData.cmd.prjData.public.runSpeed = appStateManager.globalData.cmd.textData.runSpeed,

                            appStateManager.globalData.cmd.textData.txDist = clampOrDefault(Math.round(extractHexValue(8, 1, mainCommandData) / 255 * 100), 10, 100, 60),

                            appStateManager.globalData.cmd.prjData.public.rdMode = clampOrDefault(extractHexValue(9, 1, mainCommandData), 0, 255, 0),
                            appStateManager.globalData.cmd.prjData.public.soundVal = clampOrDefault(Math.round(extractHexValue(10, 1, mainCommandData) / 255 * 100), 0, 255, 0),

                            appStateManager.globalData.cmd.textData.txPointTime = clampOrDefault(extractHexValue(15, 1, mainCommandData), 0, 100, 50),

                            appStateManager.globalData.cmd.drawData.pisObj.txPointTime = clampOrDefault(extractHexValue(16, 1, mainCommandData), 0, 100, 50),
                            appStateManager.globalData.cmd.textData.refresh = !0;
                        var projectItems = appStateManager.globalData.cmd.prjData.prjItem,
                            projectItemStartIndex = 17;
                        for (var itemKey in projectItems) {
                            var projectItem = projectItems[itemKey];
                            projectItem.pyMode = clampOrDefault(extractHexValue(projectItemStartIndex, 1, mainCommandData), 0, 255, 0),
                                projectItem.prjSelected[3] = extractHexValue(projectItemStartIndex + 1, 2, mainCommandData),
                                projectItem.prjSelected[2] = extractHexValue(projectItemStartIndex + 3, 2, mainCommandData),
                                projectItem.prjSelected[1] = extractHexValue(projectItemStartIndex + 5, 2, mainCommandData),
                                projectItem.prjSelected[0] = extractHexValue(projectItemStartIndex + 7, 2, mainCommandData),
                                projectItemStartIndex += 9
                        }
                        appStateManager.globalData.cmd.textData.runDir = clampOrDefault(extractHexValue(projectItemStartIndex, 1, mainCommandData), 0, 255, 0),
                            projectItemStartIndex += 1;
                        for (var p = appStateManager.globalData.cmd.subsetData, d = 0; d < 6; d++)
                            0 == d ? p.xyCnf.auto = p.xyCnf.autoValue == clampOrDefault(extractHexValue(projectItemStartIndex + d, 1, mainCommandData), 0, 255, 0)
                                : 1 == d ? p.xyCnf.phase = clampOrDefault(extractHexValue(projectItemStartIndex + d, 1, mainCommandData), 0, 255, 0)
                                    : p.xyCnf.xy[d - 2].value = clampOrDefault(extractHexValue(projectItemStartIndex + d, 1, mainCommandData), 0, 255, 0);

                        var settingCommandData = deviceCommandUtils.getCmdValue("00010203", "04050607", deviceResponseData);
                        appStateManager.globalData.cmd.settingData.valArr[0] = clampOrDefault(extractHexValue(1, 2, settingCommandData), 1, 512, 1),
                            appStateManager.globalData.cmd.settingData.ch = extractHexValue(3, 1, settingCommandData),
                            appStateManager.globalData.cmd.settingData.valArr[1] = clampOrDefault(extractHexValue(4, 1, settingCommandData), 10, 100, 10),
                            appStateManager.globalData.cmd.settingData.xy = clampOrDefault(extractHexValue(5, 1, settingCommandData), 0, 7, 0),
                            appStateManager.globalData.cmd.settingData.valArr[2] = clampOrDefault(extractHexValue(6, 1, settingCommandData), 0, 255, 255),
                            appStateManager.globalData.cmd.settingData.valArr[3] = clampOrDefault(extractHexValue(7, 1, settingCommandData), 0, 255, 255),
                            appStateManager.globalData.cmd.settingData.valArr[4] = clampOrDefault(extractHexValue(8, 1, settingCommandData), 0, 255, 255),
                            appStateManager.globalData.cmd.settingData.light = clampOrDefault(extractHexValue(9, 1, settingCommandData), 1, 3, 3),
                            appStateManager.globalData.cmd.settingData.cfg = clampOrDefault(extractHexValue(10, 1, settingCommandData), 0, 255, 0);

                        var featureCommandData = deviceCommandUtils.getCmdValue("D0D1D2D3", "D4D5D6D7", deviceResponseData);
                        if ("" != featureCommandData) {
                            var j = appStateManager.globalData.getDeviceFeatures(),
                                x = 16;
                            t("log", "features", JSON.stringify(j), " at utils/bluCtrl.js:96"),
                                deviceCommandUtils.getFeaturesValue({
                                    features: j
                                }, "xyCnf") && (x = 22);
                            for (var featureConfigList = [], f = clampOrDefault(extractHexValue(1, 1, featureCommandData), 0, 255, 0), F = 127 & f, k = 0; k < F; k++) {
                                for (var m = {
                                    playTime: 0,
                                    cnfValus: []
                                }, P = 0; P < x; P++) {
                                    var u = clampOrDefault(extractHexValue(3 + k * x + P, 1, featureCommandData), 0, 255, 0);
                                    m.cnfValus.push(u), 13 == P && (m.playTime = (u / 10).toFixed())
                                }
                                t("log", "pis.cnfValus", JSON.stringify(m.cnfValus), " at utils/bluCtrl.js:111"), featureConfigList.push(m)
                            }
                            appStateManager.globalData.cmd.pgsData.pisList = featureConfigList
                        }

                        var drawConfigData = deviceCommandUtils.getCmdValue("F0F1F2F3", "F4F5F6F7", deviceResponseData);
                        if ("" != drawConfigData)
                            for (var drawConfigObject = appStateManager.globalData.cmd.drawData.pisObj, configIndex = 0; configIndex < 15; configIndex++) {
                                var drawConfigParam = clampOrDefault(extractHexValue(configIndex + 1, 1, drawConfigData), 0, 255, 0);
                                configIndex < drawConfigObject.cnfValus.length && (drawConfigObject.cnfValus[configIndex] = drawConfigParam), 14 == configIndex && (drawConfigObject.txPointTime = drawConfigParam)
                            }
                    }
                }


            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "textPlaybackPageComponent ": function (e, t, r) {
            "use strict";
            (function (e) {
                var n = r("esModuleInteropHelper");
                Object.defineProperty(t, "__esModule", {
                    value: !0
                }), t.default = void 0;
                var h = n(r("uniPopupComponentExportWrapper")),
                    app = getApp(),
                    deviceCommandUtils = r("deviceCommandUtils"),
                    bleManager = r("bleDeviceControlUtils"),
                    handwritingCanvasHelper = r("handwritingCanvasHelper"),
                    handDrawFileManager = r("handDrawFileManager"),
                    codePointAt = r("codePointAt"),
                    textLineVectorizer = r("textLineVectorizer"),
                    fontGeometryUtils = r("fontGeometryUtils"),
                    b = {
                        data: function () {
                            var fontIndex = 0 | app.globalData.readData("text_fontIdex"),
                                t = app.globalData.getDeviceFeatures(),
                                r = 650 * app.globalData.screen_width_float,
                                textObject = [{
                                    text: "",
                                    update: 0,
                                    color: 9,
                                    fontIdex: fontIndex,
                                    time: 5,
                                    xys: [],
                                    XysRight: [],
                                    XysUp: [],
                                    XysDown: []
                                }];
                            return {
                                screen_width: app.globalData.screen_width_str,
                                scUnit: app.globalData.screen_width_float,
                                pageWidth: app.globalData.screen_width_page,
                                rtl: app.globalData.rtl,
                                ntitle: this.$t("Text playback"),
                                inputNote: this.$t("Please enter text"),
                                fontIdex: fontIndex,
                                fontNameList: [],
                                fontLoadIdex: -1,
                                showChineseWarn: !1,
                                canvasShow: !0,
                                sendColorTag: !0,
                                lastSendTxtCmdTime: 0,
                                lastSendTxtCmdComplete: !0,
                                popupTimeIndex: 0,
                                showSending: !1,
                                needRefresh: !0,
                                firstShow: !0,
                                textRv: !0,
                                textInput: !1,
                                timeInput0: !1,
                                timeInput1: !1,
                                timeInput2: !1,
                                timeInput3: !1,
                                inputCursor0: !1,
                                inputCursor1: !1,
                                inputCursor2: !1,
                                inputCursor3: !1,
                                maxStrCount: 0,
                                features: t,
                                currSelectedFile: "",
                                colorDisplayOrder: [{
                                    name: "Red",
                                    color: "red",
                                    order: 0,
                                    idx: 1
                                }, {
                                    name: "yellow",
                                    color: "yellow",
                                    order: 1,
                                    idx: 4
                                }, {
                                    name: "green",
                                    color: "green",
                                    order: 2,
                                    idx: 2
                                }, {
                                    name: "Cyan",
                                    color: "#00FFFF",
                                    order: 3,
                                    idx: 5
                                }, {
                                    name: "blue",
                                    color: "blue",
                                    order: 4,
                                    idx: 3
                                }, {
                                    name: "purple",
                                    color: "purple",
                                    order: 5,
                                    idx: 6
                                }, {
                                    name: "white",
                                    color: "white",
                                    order: 6,
                                    idx: 7
                                }, {
                                    name: "Jump",
                                    color: "transparent",
                                    order: 7,
                                    idx: 8
                                }, {
                                    name: "RGB",
                                    color: "transparent",
                                    order: 8,
                                    idx: 9
                                }],
                                defGroupList: textObject,
                                maxChar: 100, // max charcters
                                maxPoints: 2048, // max points
                                textData: {
                                    verTag: 0,
                                    runDir: 0,
                                    arrColor: ["red", "green", "blue", "yellow", "#00FFFF", "purple", "white"],
                                    txPointTime: 50,
                                    txColor: 9, // text color
                                    txSize: 50, // text size
                                    txDist: 50, // text distance
                                    runSpeed: 50, // run speed
                                    groupIdex: 0,
                                    // [the groupList: textObject array contains objects with the following structure]
                                    //text: "",         // The actual text string for this group
                                    //update: 0,        // A flag indicating if the group needs to be updated/redrawn
                                    //color: 9,         // Color index for the text
                                    //fontIdex: <int>,  // Index of the font to use for rendering
                                    // time: 5,          // Duration or timing value for playback
                                    //xys: [],          // Array of coordinate points for the text's shape


                                    //XysRight: [],     // Optional: coordinates for rightward animation
                                    // XysUp: [],        // Optional: coordinates for upward animation
                                    //XysDown: []       // Optional: coordinates for downward animation
                                    //}
                                    //
                                    groupList: textObject
                                },
                                position: {
                                    x: r,
                                    y: r
                                },
                                startPosition: {
                                    x: 0,
                                    y: 0
                                }
                            }
                        },
                        onLoad: function () {
                            var fontRegistryModule = r("fontRegistryModule ");
                            this.fontNameList = fontRegistryModule.getFontNameList();
                            var textData = app.globalData.getCmdData("textData");
                            this.textData = textData;
                            for (var n = 0; n < this.textData.groupList.length; n++)
                                null == this.textData.groupList[n].fontIdex
                                    && (this.textData.groupList[n].fontIdex = this.fontIdex)
                        },

                        computed: {
                            textTime: {
                                get: function () {
                                    for (var e = [], t = 0; t < this.textData.groupList.length; t++) {
                                        var r = parseFloat(this.textData.groupList[t].time);
                                        this.features.textDecimalTime ? e.push(r.toFixed(1)) : e.push(r.toFixed(0))
                                    }
                                    return e
                                }
                            }
                        },
                        methods: {
                            // send text command
                            sendTxtCmd: function () {
                                var e = this;
                                app.globalData.setCmdData("textData", this.textData);
                                var runDir = this.textData.runDir,
                                    command = deviceCommandUtils.getXysCmdArr(
                                        this.textData.groupList,
                                        this.features, runDir,
                                        this.textData.verTag),
                                    n = this;
                                bleManager.gosend(!0, command);

                            },

                            sendCmd: function () {
                                app.globalData.setCmdData("textData", this.textData);
                                var command = deviceCommandUtils.getCmdStr(
                                    app.globalData.cmd, {
                                    features: this.features,
                                    groupList: this.textData.groupList
                                });
                                bleManager.gosend(!1, command) && (this.sendColorTag = !1)
                            },
                            sendRvXysCmd: function () {
                                var settingData = app.globalData.cmd.settingData;
                                0 == this.textData.runDir ? settingData.xy = 0 : settingData.xy = 3;
                                var settingCommand = deviceCommandUtils.getSettingCmd(app.globalData.cmd.settingData);
                                bleManager.gosend(!1, settingCommand) && this.sendCmdBtn(null)
                            },
                            readFontBase64: function (fontIdex) {
                                var t = this,
                                    callback = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null;
                                if (this.fontLoadIdex != fontIdex) {
                                    var fontRegistryModule = r("fontRegistryModule "),
                                        fontList = fontRegistryModule.getFontList(this),
                                        file = fontList[fontIdex].file,
                                        mode = fontList[fontIdex].mode,
                                        sn = fontList[fontIdex].sn;
                                    fontGeometryUtils.readTTF(file, mode, (function (data, mode) {
                                        fontGeometryUtils.fontData = {
                                            data: data,
                                            mode: mode,
                                            sn: sn
                                        }, t.fontLoadIdex = fontIdex, callback && callback()
                                    }))
                                } else callback && callback()
                            },
                            setFontIdex: function (e) {
                                this.fontIdex != e && (this.fontIdex = e, app.globalData.saveData("text_fontIdex", this.fontIdex))
                            },
                            onFontChange: function (e) {
                                var t = e.detail.value;
                                this.setFontIdex(t), this.textData.groupList[this.textData.groupIdex].fontIdex = t,
                                    this.textData.groupList[this.textData.groupIdex].update = 1
                            },
                            slPointTimeChange: function (e) {
                                var t = e.detail.value;
                                this.textData.txPointTime = t,
                                    this.sendCmd()
                            },
                            slTxSizeChange: function (e) {
                                var t = e.detail.value;
                                this.textData.txSize = t,
                                    handwritingCanvasHelper.doDrawPicEx(this), this.sendCmd()
                            },
                            btnColorChange: function (e) {
                                var t = parseInt(e.currentTarget.dataset.tag);
                                this.textData.groupList[this.textData.groupIdex].color = t,
                                    this.$set(this.textData, "txColor", t),
                                    handwritingCanvasHelper.doDrawPicEx(this), this.sendCmd()
                            },
                            slTxDistChange: function (e) {
                                var t = e.detail.value;
                                this.textData.txDist = t,
                                    this.sendCmd()
                            },
                            slRunSpeedChange: function (e) {
                                var t = e.detail.value;
                                this.textData.runSpeed = t,
                                    this.sendCmd()
                            },
                            radioRunDirectionChange: function (e) {
                                var t = e.detail.value;
                                "textUp" == t ? this.$set(this.textData, "runDir", 127) : "textDown" == t
                                    ? this.$set(this.textData, "runDir", 128)
                                    : "right" == t
                                        ? this.$set(this.textData, "runDir", 255)
                                        : this.$set(this.textData, "runDir", 0),
                                    this.sendColorTag = !0, this.sendCmdBtn(null)
                            },
                            radioDisplayChange: function (e) {
                                var t = e.detail.value;
                                "h" == t
                                    ? this.$set(this.textData, "verTag", 0)
                                    : this.$set(this.textData, "verTag", 255)
                            },
                            createXys: function (inputText) {
                                var textPlaybackPageComponent = this,
                                    callback = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null,
                                    fallBackShapes = [
                                        [0, [{
                                            x: -194,
                                            y: 129,
                                            z: 1
                                        }, {
                                            x: -119,
                                            y: 153,
                                            z: 0
                                        }, {
                                            x: -100,
                                            y: 174,
                                            z: 1
                                        }], 234, 234],
                                        [0, [{
                                            x: -111,
                                            y: -105,
                                            z: 1
                                        }, {
                                            x: -42,
                                            y: -105,
                                            z: 1
                                        }], 234, 234],
                                        [0, [{
                                            x: -114,
                                            y: 153,
                                            z: 1
                                        }, {
                                            x: -114,
                                            y: -105,
                                            z: 1
                                        }, {
                                            x: -186,
                                            y: -108,
                                            z: 1
                                        }], 234, 234],
                                        [1, [{
                                            x: 189,
                                            y: -105,
                                            z: 1
                                        }, {
                                            x: 50,
                                            y: -102,
                                            z: 1
                                        }, {
                                            x: 58,
                                            y: -76,
                                            z: 0
                                        }, {
                                            x: 170,
                                            y: 44,
                                            z: 0
                                        }, {
                                            x: 186,
                                            y: 89,
                                            z: 0
                                        }, {
                                            x: 176,
                                            y: 142,
                                            z: 0
                                        }, {
                                            x: 149,
                                            y: 164,
                                            z: 0
                                        }, {
                                            x: 90,
                                            y: 166,
                                            z: 0
                                        }, {
                                            x: 53,
                                            y: 150,
                                            z: 0
                                        }, {
                                            x: 37,
                                            y: 121,
                                            z: 1
                                        }], 234, 234]
                                    ];
                                fallBackShapes = [
                                    [0, [{
                                        x: -216,
                                        y: 174,
                                        z: 1
                                    }, {
                                        x: -219,
                                        y: 88,
                                        z: 1
                                    }, {
                                        x: -366,
                                        y: 88,
                                        z: 1
                                    }, {
                                        x: -366,
                                        y: -88,
                                        z: 1
                                    }, {
                                        x: -339,
                                        y: -88,
                                        z: 1
                                    }, {
                                        x: -336,
                                        y: -66,
                                        z: 1
                                    }, {
                                        x: -219,
                                        y: -66,
                                        z: 1
                                    }, {
                                        x: -216,
                                        y: -210,
                                        z: 1
                                    }, {
                                        x: -187,
                                        y: -210,
                                        z: 1
                                    }, {
                                        x: -184,
                                        y: -66,
                                        z: 1
                                    }, {
                                        x: -64,
                                        y: -66,
                                        z: 1
                                    }, {
                                        x: -62,
                                        y: -88,
                                        z: 1
                                    }, {
                                        x: -35,
                                        y: -88,
                                        z: 1
                                    }, {
                                        x: -35,
                                        y: 88,
                                        z: 1
                                    }, {
                                        x: -184,
                                        y: 88,
                                        z: 1
                                    }, {
                                        x: -187,
                                        y: 174,
                                        z: 1
                                    }, {
                                        x: -216,
                                        y: 174,
                                        z: 0
                                    }], 400, 400],
                                    [0, [{
                                        x: -187,
                                        y: 59,
                                        z: 1
                                    }, {
                                        x: -64,
                                        y: 62,
                                        z: 1
                                    }, {
                                        x: -62,
                                        y: -37,
                                        z: 1
                                    }, {
                                        x: -184,
                                        y: -40,
                                        z: 1
                                    }, {
                                        x: -187,
                                        y: 59,
                                        z: 0
                                    }], 400, 400],
                                    [0, [{
                                        x: -339,
                                        y: 59,
                                        z: 1
                                    }, {
                                        x: -219,
                                        y: 62,
                                        z: 1
                                    }, {
                                        x: -216,
                                        y: -37,
                                        z: 1
                                    }, {
                                        x: -336,
                                        y: -40,
                                        z: 1
                                    }, {
                                        x: -339,
                                        y: 59,
                                        z: 0
                                    }], 400, 400],
                                    [1, [{
                                        x: 29,
                                        y: 152,
                                        z: 1
                                    }, {
                                        x: 29,
                                        y: -208,
                                        z: 1
                                    }, {
                                        x: 56,
                                        y: -208,
                                        z: 1
                                    }, {
                                        x: 58,
                                        y: -189,
                                        z: 1
                                    }, {
                                        x: 338,
                                        y: -189,
                                        z: 1
                                    }, {
                                        x: 341,
                                        y: -208,
                                        z: 1
                                    }, {
                                        x: 368,
                                        y: -208,
                                        z: 1
                                    }, {
                                        x: 368,
                                        y: 152,
                                        z: 1
                                    }, {
                                        x: 29,
                                        y: 152,
                                        z: 0
                                    }], 400, 400],
                                    [1, [{
                                        x: 58,
                                        y: 128,
                                        z: 1
                                    }, {
                                        x: 341,
                                        y: 126,
                                        z: 1
                                    }, {
                                        x: 338,
                                        y: -165,
                                        z: 1
                                    }, {
                                        x: 56,
                                        y: -162,
                                        z: 1
                                    }, {
                                        x: 58,
                                        y: 128,
                                        z: 0
                                    }], 400, 400],
                                    [1, [{
                                        x: 253,
                                        y: -32,
                                        z: 1
                                    }, {
                                        x: 237,
                                        y: -45,
                                        z: 1
                                    }, {
                                        x: 272,
                                        y: -82,
                                        z: 1
                                    }, {
                                        x: 288,
                                        y: -66,
                                        z: 1
                                    }, {
                                        x: 253,
                                        y: -32,
                                        z: 0
                                    }], 400, 400],
                                    [1, [{
                                        x: 85,
                                        y: 86,
                                        z: 1
                                    }, {
                                        x: 85,
                                        y: 62,
                                        z: 1
                                    }, {
                                        x: 186,
                                        y: 59,
                                        z: 1
                                    }, {
                                        x: 184,
                                        y: 6,
                                        z: 1
                                    }, {
                                        x: 90,
                                        y: 6,
                                        z: 1
                                    }, {
                                        x: 90,
                                        y: -18,
                                        z: 1
                                    }, {
                                        x: 186,
                                        y: -21,
                                        z: 1
                                    }, {
                                        x: 186,
                                        y: -93,
                                        z: 1
                                    }, {
                                        x: 77,
                                        y: -96,
                                        z: 1
                                    }, {
                                        x: 77,
                                        y: -120,
                                        z: 1
                                    }, {
                                        x: 320,
                                        y: -120,
                                        z: 1
                                    }, {
                                        x: 320,
                                        y: -96,
                                        z: 1
                                    }, {
                                        x: 210,
                                        y: -93,
                                        z: 1
                                    }, {
                                        x: 213,
                                        y: -18,
                                        z: 1
                                    }, {
                                        x: 304,
                                        y: -18,
                                        z: 1
                                    }, {
                                        x: 304,
                                        y: 6,
                                        z: 1
                                    }, {
                                        x: 213,
                                        y: 6,
                                        z: 1
                                    }, {
                                        x: 210,
                                        y: 59,
                                        z: 1
                                    }, {
                                        x: 312,
                                        y: 62,
                                        z: 1
                                    }, {
                                        x: 312,
                                        y: 86,
                                        z: 1
                                    }, {
                                        x: 85,
                                        y: 86,
                                        z: 0
                                    }], 400, 400]
                                ], fallBackShapes = [
                                    [0, [{
                                        x: 170,
                                        y: 112,
                                        z: 1
                                    }, {
                                        x: 141,
                                        y: 142,
                                        z: 1
                                    }, {
                                        x: 117,
                                        y: 118,
                                        z: 0
                                    }, {
                                        x: -112,
                                        y: 118,
                                        z: 0
                                    }, {
                                        x: -136,
                                        y: 131,
                                        z: 1
                                    }, {
                                        x: -136,
                                        y: -42,
                                        z: 0
                                    }, {
                                        x: -155,
                                        y: -120,
                                        z: 0
                                    }, {
                                        x: -184,
                                        y: -176,
                                        z: 1
                                    }, {
                                        x: -152,
                                        y: -136,
                                        z: 0
                                    }, {
                                        x: -126,
                                        y: -72,
                                        z: 0
                                    }, {
                                        x: -115,
                                        y: 112,
                                        z: 1
                                    }, {
                                        x: 170,
                                        y: 112,
                                        z: 0
                                    }], 400, 400],
                                    [0, [{
                                        x: -16,
                                        y: 182,
                                        z: 1
                                    }, {
                                        x: 16,
                                        y: 126,
                                        z: 1
                                    }, {
                                        x: 24,
                                        y: 158,
                                        z: 1
                                    }, {
                                        x: -16,
                                        y: 182,
                                        z: 0
                                    }], 400, 400]
                                ];
                                var cleanedInput = inputText.replace("\n", "");
                                "" != cleanedInput ? (uni.showLoading({
                                    title: this.$t("Generating coordinate points..."),
                                    mask: !0
                                }), this.readFontBase64(this.fontIdex, (function () {
                                    var textCoordinates = textLineVectorizer.getXXYY(codePointAt, fontGeometryUtils.fontData, cleanedInput, textPlaybackPageComponent.textRv);
                                    uni.hideLoading(),
                                        fallBackShapes = textCoordinates.xxyy,
                                        fontGeometryUtils.ifHasChinese(textCoordinates.notRec)
                                        && 1001 == fontGeometryUtils.fontData.sn
                                        && app.globalData.showModalTips(textPlaybackPageComponent.$t("Due to capacity limitations, some Chinese characters are not included in the font library. For the complete font library, please refer to the APP version"), !0);
                                    var i = textPlaybackPageComponent.getSumSizeExclude(textPlaybackPageComponent.textData.groupIdex),
                                        c = handwritingCanvasHelper.getTxXySize(fallBackShapes);
                                    c.chCount + i.chCount > textPlaybackPageComponent.maxChar
                                        ? app.globalData.showModalTips(textPlaybackPageComponent.$t("The number of text coordinate points has exceeded 2048, please re-enter."), !0)
                                        : c.ptCount + i.ptCount > textPlaybackPageComponent.maxPoints
                                            ? app.globalData.showModalTips(textPlaybackPageComponent.$t("The number of text coordinate points has exceeded 2048, please re-enter."), !0)
                                            : callback && callback(fallBackShapes, textCoordinates.xxyyRight, textCoordinates.xxyyUp, textCoordinates.xxyyDown)
                                }))) : callback && callback([])
                            },
                            getSumSizeExclude: function () {
                                for (var excludedIndex = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null, charCount = 0, pointCount = 0, n = 0; n < this.textData.groupList.length; n++)
                                    if (null == excludedIndex || n != excludedIndex) {
                                        var h = handwritingCanvasHelper.getTxXySize(this.textData.groupList[n].xys);
                                        charCount += h.chCount, pointCount += h.ptCount
                                    } return {
                                        chCount: charCount,
                                        ptCount: pointCount
                                    }
                            },
                            inputEvent: function (e) {
                                var t = e.detail.value;
                                this.textData.groupList[this.textData.groupIdex].update = 1,
                                    this.$set(this.textData.groupList[this.textData.groupIdex], "text", t)
                            },
                            onTimeBlur: function (t) {
                                var r = this.textData.groupList[t].time;
                                e("log", "time", r, " at sub/pages/text/text.js:973"),
                                    this.features.textDecimalTime ? r < 1 || r > 25.5 ? (app.globalData.showModalTips(this.$t("text_time_range"), !0), this.$set(this.textData.groupList[t], "time", 5)) : this.$set(this.textData.groupList[t], "time", Math.floor(10 * r) / 10) : r < 1 || r > 255 ? (app.globalData.showModalTips(this.$t("\u8bf7\u8f93\u51651-255\u8303\u56f4\u7684\u6570\u503c"), !0), this.$set(this.textData.groupList[t], "time", 5)) : this.$set(this.textData.groupList[t], "time", Math.floor(r))
                            },
                            onGroupChange: function (e) {
                                this.textData.groupIdex != e.detail.value && (this.$set(this.textData, "groupIdex", e.detail.value), handwritingCanvasHelper.doDrawPicEx(this))
                            },
                            refreshCanvasDraw: function () {
                                var e = this;
                                this.$nextTick((function () {
                                    var t = uni.createSelectorQuery().in(e);
                                    t.select("#myCanvas").boundingClientRect((function (t) {
                                        e.cvWH = {
                                            w: t.width,
                                            h: t.height
                                        }, setTimeout((function () {
                                            handwritingCanvasHelper.doDrawPicEx(e)
                                        }), 100)
                                    })).exec()
                                }))
                            },
                            createXyByIdex: function (index) {
                                var t = this,
                                    r = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : null;
                                if (0 != this.textData.groupList[index].update) {
                                    var text = this.textData.groupList[index].text;
                                    this.createXys(text, (function (mainCoords, rightCoords, upCoords, downCoords) {
                                        if (0 == mainCoords.length) return app.globalData.showModalTips(t.inputNote, !0), void (r && r());
                                        t.textData.groupList[index].xys = mainCoords,
                                            t.textData.groupList[index].XysRight = rightCoords,
                                            t.textData.groupList[index].XysUp = upCoords,
                                            t.textData.groupList[index].XysDown = downCoords,
                                            t.textData.groupList[index].update = 0,
                                            r && r()
                                    }))
                                } else r && r()
                            },
                            checkCurrentGroupOk: function () {
                                var e = this.textData.groupList[this.textData.groupIdex].xys,
                                    t = this.textData.groupList[this.textData.groupIdex].text,
                                    r = this.textData.groupList[this.textData.groupIdex].fontIdex,
                                    n = !0,
                                    h = "";
                                if (1007 == this.fontNameList[r].sn) return !0;
                                for (var i = 0; i < e.length; i++) {
                                    var c = e[i][0];
                                    if (e[i][1].length <= 1 && " " != t[c]) {
                                        for (var o = !0, s = 0; s < h.length; s++)
                                            if (h[s] == t[c]) {
                                                o = !1;
                                                break
                                            } o && (h += t[c]), n = !1
                                    }
                                }
                                if (!n) {
                                    var l = this.fontNameList[r].name;
                                    return h.length > 4 && (h = h.substring(0, 3) + "..."),
                                        app.globalData.showModalTips(this.$t("\u7b2c") + (this.textData.groupIdex + 1) + this.$t("\u7ec4\u5b57\u4f53") + this.$t(l) + this.$t("\u4e0d\u652f\u6301\u6587\u672c") + '"' + h + '",' + this.$t("\u8bf7\u4fee\u6539\u5b57\u4f53\u6216\u6587\u672c\u540e\u91cd\u8bd5"), !0), !1
                                }
                                return !0
                            },
                            addGroup: function (e) {
                                var t = this;
                                if (this.textData.groupList.length >= 4) app.globalData.showModalTips(this.$t("\u6700\u591a4\u4e2a\u5206\u7ec4"), !0);
                                else if ("" != this.textData.groupList[this.textData.groupIdex].text.trim()) {
                                    var r = this;
                                    this.createXyByIdex(this.textData.groupIdex, (function () {
                                        "" != t.textData.groupList[t.textData.groupIdex].text.trim()
                                            ? r.checkCurrentGroupOk() && (r.textData.groupList.push({
                                                text: "",
                                                time: 5,
                                                color: 9,
                                                update: 0,
                                                fontIdex: r.fontIdex,
                                                xys: []
                                            }), r.$set(r.textData, "groupIdex", r.textData.groupList.length - 1),
                                                r.$set(t.textData, "txColor", r.textData.groupList[r.textData.groupIdex].color),
                                                r.refreshCanvasDraw(), r.sendColorTag = !0, r.textInput = !1, setTimeout((function () {
                                                    r.textInput = !0
                                                }), 100)) : app.globalData.showModalTips(t.inputNote, !0)
                                    }))
                                } else app.globalData.showModalTips(this.inputNote, !0)
                            },
                            oprEdit: function (e) {
                                var t = this;
                                this.textData.groupIdex !== e && ("" != this.textData.groupList[this.textData.groupIdex].text.trim() ? this.createXyByIdex(this.textData.groupIdex, (function () {
                                    t.checkCurrentGroupOk() && (t.$set(t.textData, "groupIdex", e), t.setFontIdex(t.textData.groupList[t.textData.groupIdex].fontIdex), t.$set(t.textData, "txColor", t.textData.groupList[t.textData.groupIdex].color), t.refreshCanvasDraw())
                                })) : app.globalData.showModalTips(this.inputNote, !0))
                            },
                            changeTimeClick: function (e, t) {
                                var r = 0;
                                this.features.textDecimalTime ? (r = parseFloat(this.textData.groupList[t].time), r += .1 * e, r < 1 && (r = 1), r > 25.5 && (r = 25.5), r = Math.round(10 * r) / 10) : (r = parseInt(this.textData.groupList[t].time), r += e, r < 1 && (r = 1), r > 255 && (r = 255)), this.$set(this.textData.groupList[t], "time", r)
                            },
                            setTimeInput: function (e) {
                                this.popupTimeIndex = e, this.$refs.popupTime.open("bottom")
                            },
                            previwBtn: function (e) {
                                var t = this;
                                this.createXyByIdex(this.textData.groupIdex, (function () {
                                    handwritingCanvasHelper.doDrawPicEx(t)
                                }))
                            },
                            sendCmdBtn: function (e) {
                                var t = this;
                                this.lastSendTxtCmdComplete
                                    && this.createXyByIdex(this.textData.groupIdex, (function () {
                                        "" != t.textData.groupList[t.textData.groupIdex].text.trim()
                                            ? t.checkCurrentGroupOk() && (t.sendColorTag && t.sendCmd(),
                                                t.lastSendTxtCmdComplete = !1,
                                                t.sendTextCmdMustOk((new Date).getTime()),
                                                handwritingCanvasHelper.doDrawPicEx(t))
                                            : app.globalData.showModalTips(t.inputNote, !0)
                                    }))
                            },

                            restoreDeskTop: function () {
                                var t = handDrawFileManager.getTextFileData("saveDeskTopFile_002", !0);
                                if (t) {
                                    var r = t.data.features,
                                        n = this.features;
                                    if (r.textDecimalTime != n.textDecimalTime || r.textModeFix01 != n.textModeFix01 || !(n.textUpDown && r.hasOwnProperty("textUpDown") && r.textUpDown) && n.textUpDown) return e("log", "\u5f53\u524d\u4fdd\u5b58\u7684\u6587\u672c\u683c\u5f0f\u4e0d\u652f\u6301", " at sub/pages/text/text.js:1211"), this.textData.groupList = this.defGroupList, void (this.textData.groupIdex = 0);
                                    app.globalData.cmd.textData.refresh ? (app.globalData.cmd.textData.refresh = !1, this.textData.groupIdex = t.data.textData.groupIdex, this.textData.groupList = t.data.textData.groupList) : this.textData = t.data.textData;
                                    for (var h = 0; h < this.textData.groupList.length; h++) null == this.textData.groupList[h].fontIdex && (this.textData.groupList[h].fontIdex = this.fontIdex);
                                    this.textData.groupList.length > 0 && this.setFontIdex(this.textData.groupList[this.textData.groupIdex].fontIdex), this.sendColorTag = !0, this.needRefresh = !0
                                }
                            },
                            saveTextFileData: function (e) {
                                var t = this;
                                this.createXyByIdex(this.textData.groupIdex, (function () {
                                    if ("" == t.textData.groupList[t.textData.groupIdex].text.trim()) return app.globalData.showModalTips(t.$t("\u7b2c") + (t.textData.groupIdex + 1) + t.$t("\u7ec4\u6587\u672c\u4e3a\u7a7a\uff0c\u8bf7\u8f93\u5165\u518d\u4fdd\u5b58"), !0), !1;
                                    if (t.checkCurrentGroupOk()) {
                                        handwritingCanvasHelper.doDrawPicEx(t);
                                        var r = t.getSumSizeExclude(),
                                            n = {
                                                features: t.features,
                                                textData: t.textData
                                            };
                                        handDrawFileManager.saveTextFileData(e, n, r), t.currSelectedFile = e, app.globalData.showModalTips(t.$t("\u4fdd\u5b58\u6210\u529f"))
                                    }
                                }))
                            },
                            getFileDataByName: function (e) {
                                var t = handDrawFileManager.getTextFileData(e);
                                if (t) {
                                    this.textData = t.data.textData;
                                    var r = !1;
                                    this.textData.hasOwnProperty("runDir") || (this.textData["runDir"] = 0, r = this.features.arbPlay);
                                    for (var n = 0; n < this.textData.groupList.length; n++) null == this.textData.groupList[n].fontIdex && (this.textData.groupList[n].fontIdex = this.fontIdex), r && (this.textData.groupList[n].update = 1);
                                    this.textData.groupList.length > 0 && this.setFontIdex(this.textData.groupList[this.textData.groupIdex].fontIdex), this.sendColorTag = !0, this.needRefresh = !0, this.currSelectedFile = e
                                }
                            },

                        }
                    };
                t.default = b
            }).call(this, r("enhancedConsoleLogger")["default"])


        },

        "handwritingCanvasHelper": function (e, t) {
            function getTxXySize(xysArray) {
                for (var currentCharId = -1, pointCount = 0, charCount = 0, index = 0; index < xysArray.length; index++) {
                    var currentItem = xysArray[index];
                    currentCharId != currentItem[0] && (currentCharId = currentItem[0], charCount++), pointCount += currentItem[1].length
                }
                return {
                    chCount: charCount,
                    ptCount: pointCount
                }
            }

            function doDrawPicEx(textComponenet) {
                var textCompnenet2 = textComponenet;
                (function (textCompnenet3) {
                    var textPlaybackPageComponent = textCompnenet3,
                        textData = textPlaybackPageComponent.textData,
                        xyData = 0 == textData.groupList.length ? null : textData.groupList[textData.groupIdex].xys;
                    if (null != xyData) {
                        for (var a = textData.txColor, i = textData.arrColor, c = textPlaybackPageComponent.cvWH.w / 2, o = textPlaybackPageComponent.cvWH.h / 2, s = textPlaybackPageComponent.ctx, l = textData.txSize / 100, p = textPlaybackPageComponent.cvWH.h / 800 * l, d = "red", b = -1, g = -1, charPointCount = getTxXySize(xyData), x = 0; x < xyData.length; x++)
                            if (!(xyData[x][1].length < 2)) {
                                b != xyData[x][0] && (g++, s.beginPath(), b = xyData[x][0], d = a <= 7 ? i[a - 1] : i[g % 7], s.strokeStyle = d);
                                for (var V = xyData[x][1], f = 0; f < V.length; f++) {
                                    var F = V[f],
                                        k = F.x * p + c,
                                        m = o - F.y * p;
                                    0 == f ? s.moveTo(k.toFixed(), m.toFixed()) : Math.abs(F.x - V[f - 1].x) < 1 && Math.abs(F.y - V[f - 1].y) < 1 ? (s.arc(k, m, 1, 0, 2 * Math.PI), s.moveTo(k.toFixed(), m.toFixed())) : s.lineTo(k.toFixed(), m.toFixed())
                                }
                                s.stroke()
                            } s.beginPath(), s.setFillStyle("white"), s.setFontSize(10);
                        var P = textPlaybackPageComponent.getSumSizeExclude(textData.groupIdex),
                            u = textPlaybackPageComponent.$t("Character count") + ": " + (charPointCount.chCount + P.chCount) + "/" + textPlaybackPageComponent.maxChar + "  " + textPlaybackPageComponent.$t("Point count") + ": " + (charPointCount.ptCount + P.ptCount) + "/" + textPlaybackPageComponent.maxPoints;
                        s.fillText(u, 4, textPlaybackPageComponent.cvWH.h - 4), s.draw()
                    }
                })(textCompnenet2)
            }
            e.exports = {
                getTxXySize: getTxXySize,
                doDrawPicEx: doDrawPicEx,
                onReady: function (e) {
                    var t = e,
                        r = uni.createSelectorQuery();
                    r.select("#myCanvas").fields({
                        node: !0,
                        size: !0
                    }).exec((function (e) {
                        e[0].node;
                        var r = uni.createCanvasContext("myCanvas", t);
                        t.ctx = r;
                        var h = uni.getSystemInfoSync(),
                            a = h.pixelRatio;
                        t.cvRatio = {
                            w: e[0].width * a,
                            h: e[0].height * a
                        }, t.cvWH = {
                            w: e[0].width,
                            h: e[0].height
                        }, t.$nextTick((function () {
                            doDrawPicEx(t)
                        }))
                    }))
                }
            }
        },

        "fontRegistryModule": function (e, t, r) {
            var mergedDrawFontsUtils = r("mergedDrawFontsUtils"),
                h = [{
                    name: "Single Line Font",
                    file: mergedDrawFontsUtils.DrawFonts,
                    mode: 2,
                    sn: 1004,
                    note: "font_note_1004",
                    msg: "Minimal strokes, no flicker, recommended for use"
                }, {
                    name: "SimSun",
                    file: "simsun_0.woff",
                    mode: 1,
                    sn: 1003,
                    note: "font_note_1003"
                }, {
                    name: "Source Han Sans 1",
                    file: "latin.woff",
                    mode: 1,
                    sn: 1002,
                    note: "font_note_1002"
                }, {
                    name: "Source Han Sans 2",
                    file: "china.woff",
                    mode: 1,
                    sn: 1005,
                    note: "font_note_1005"
                }, {
                    name: "Source Han Sans 3",
                    file: "japan_korea.woff",
                    mode: 1,
                    sn: 1006,
                    note: "font_note_1006"
                }, {
                    name: "Source Han Sans 4",
                    file: "arabic.woff",
                    mode: 1,
                    sn: 1007,
                    note: "font_note_1007"
                }];
            e.exports = {
                getFontNameList: function (e) {
                    for (var t = [], r = 0; r < h.length; r++) {
                        var n = h[r].name,
                            a = h[r].sn;
                        t.push({
                            name: n,
                            sn: a
                        })
                    }
                    return t
                },
                getFontList: function (e) {
                    return h
                }
            }
        },

        "fontGeometryUtils": function (e, t) {
            function calculateAngleBetweenPoints(pointA, vertex, pointB) {
                var vectorA = {
                    x: pointA[0] - vertex[0],
                    y: pointA[1] - vertex[1]
                },
                    vectorB = {
                        x: pointB[0] - vertex[0],
                        y: pointB[1] - vertex[1]
                    },
                    dotProduct = vectorA.x * vectorB.x + vectorA.y * vectorB.y,
                    magnitudeA = Math.sqrt(Math.pow(vectorA.x, 2) + Math.pow(vectorA.y, 2)),
                    magnitudeB = Math.sqrt(Math.pow(vectorB.x, 2) + Math.pow(vectorB.y, 2));
                if (0 == magnitudeA || 0 == magnitudeB) return 0;
                var o = Math.acos(dotProduct / (magnitudeA * magnitudeB)),
                    s = 180 * o / Math.PI;
                return s
            }

            function getLineRectangleIntersection(pointA, pointB, box) {
                var rectangleBoundary = {
                    w: box.w,
                    h: -box.h
                },
                    slope = (pointA[1] - pointB[1]) / (pointB[0] - pointA[0]),
                    yIntercept = -pointB[1] - slope * pointB[0],
                    intersectionPoint = [];
                yIntercept <= 0 && yIntercept >= rectangleBoundary.h && (pointB[0] < 0 || pointA[0] < 0) && (intersectionPoint = [0, -yIntercept]);
                var c = slope * rectangleBoundary.w + yIntercept;
                if (c <= 0 && c >= rectangleBoundary.h && (pointB[0] > rectangleBoundary.w || pointA[0] > rectangleBoundary.w) && (intersectionPoint = [rectangleBoundary.w, -c]), 0 != slope) {
                    var o = -yIntercept / slope;
                    o >= 0 && o <= rectangleBoundary.w && (pointB[1] < 0 || pointA[1] < 0) && (intersectionPoint = [o, 0]);
                    var s = (rectangleBoundary.h - yIntercept) / slope;
                    s >= 0 && s <= rectangleBoundary.w && (pointB[1] > -rectangleBoundary.h || pointA[1] > -rectangleBoundary.h) && (intersectionPoint = [s, -rectangleBoundary.h])
                }
                return intersectionPoint.length > 0 && intersectionPoint.push(1), intersectionPoint
            }

            function isChineese(e) {
                return /[\u4E00-\u9FA5]/.test(e)
            }
            e.exports = {
                fontData: null,
                ifHasChinese: function (e) {
                    if (null == e) return !1;
                    for (var t = 0; t < e.length; t++)
                        if (isChineese(e[t])) return !0;
                    return !1
                },
                parseLines: function (points) {
                    for (var rextangleSize = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : {
                        w: 800,
                        h: 800
                    }, r = [], h = [], a = 0; a < points.length; a++) {
                        var i = points[a];
                        if (i[0] < 0 || i[0] > rextangleSize.w || i[1] < 0 || i[1] > rextangleSize.h) {
                            if (0 != h.length) {
                                var c = getLineRectangleIntersection(h[h.length - 1], i, rextangleSize);
                                c.length > 0 && h.push(c), r.push(h), h = []
                            }
                        } else {
                            if (a > 0 && 0 == h.length) {
                                var o = points[a - 1];
                                if (o[0] < 0 || o[0] > rextangleSize.w || o[1] < 0 || o[1] > rextangleSize.h) {
                                    var s = getLineRectangleIntersection(o, i, rextangleSize);
                                    s.length > 0 && h.push(s)
                                }
                            }
                            h.push(i)
                        }
                    }
                    return h.length > 0 && r.push(h), r
                },
                readTTF: function (fontName, mode, callback) {
                    2 != mode ? plus.io.resolveLocalFileSystemURL("_www/static/app-plus/font/" + fontName, (function (e) {
                        e.file((function (e) {
                            var fileReader = new plus.io.FileReader;
                            fileReader.onloadend = function (e) {
                                var dataURl = e.target.result,
                                    fontData = dataURl.split(",")[1];
                                callback(fontData, mode)
                            }, fileReader.onerror = function () {
                                uni.hideLoading(), uni.showToast({
                                    title: "\u8bfb\u53d6\u5b57\u4f53\u5931\u8d25",
                                    icon: "none"
                                })
                            }, fileReader.readAsDataURL(e)
                        }))
                    }), (function (e) {
                        uni.hideLoading(), uni.showToast({
                            title: "\u5b57\u4f53\u6587\u4ef6\u89e3\u6790\u5931\u8d25\uff1a" + JSON.stringify(e),
                            icon: "none"
                        })
                    })) : callback(fontName, mode)
                },
                dealLine: function (points) {
                    var t = !(arguments.length > 1 && void 0 !== arguments[1]) || arguments[1],
                        output = [],
                        previousPoint = points[0];
                    output.push([previousPoint[0], previousPoint[1], 0, 1]);
                    for (var index = 1; index < points.length - 1; index++) {
                        var currentPoint = points[index],
                            nextPoint = points[index + 1],
                            angle = calculateAngleBetweenPoints(previousPoint, currentPoint, nextPoint);
                        if (0 != angle && 180 != angle) {
                            var s = angle <= 135 ? 1 : 0;
                            t || (s = 0), output.push([currentPoint[0], currentPoint[1], 1, s]), previousPoint = currentPoint
                        }
                    }
                    var l = points[points.length - 1];
                    return output.push([l[0], l[1], 1, 1]), output
                }
            }

        },

        "textLineVectorizer": function (e, t, r) {
            (function (t) {
                var spreadToArrayHelper = r("spreadToArrayHelper"),
                    arrayConversionHelper = r("arrayConversionHelper"),
                    arabicHelper = r("arabicPresentationFormsConverter");

                function sampleQuadraticBezier(start, controlPoint, endPoint, n) {
                    for (var h = [], a = 0; a <= 1; a += 1 / n) {
                        var i = Math.pow(1 - a, 2) * start.x + 2 * (1 - a) * a * controlPoint.x + Math.pow(a, 2) * endPoint.x,
                            c = Math.pow(1 - a, 2) * start.y + 2 * (1 - a) * a * controlPoint.y + Math.pow(a, 2) * endPoint.y;
                        h.push({
                            x: i,
                            y: c,
                            z: 0
                        })
                    }
                    return h
                }

                function appendToArrayOr(e, t) {
                    return e.length, e.push(t), !0
                }

                function parsePathCommands(pathCommands) {
                    for (var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 5, r = [], n = [], h = 0, index = 0; index < pathCommands.length; index++) {
                        var cmd = pathCommands[index];
                        // MOVE
                        if ("M" == cmd.type) {
                            var s = {
                                x: cmd.x,
                                y: cmd.y,
                                z: 1
                            };
                            h = appendToArrayOr(n, s) ? h : h + 1
                        }
                        //LINETO
                        if ("L" == cmd.type) {
                            var l = {
                                x: cmd.x,
                                y: cmd.y,
                                z: 1
                            };
                            h = appendToArrayOr(n, l) ? h : h + 1
                        }
                        //QUADRATICBEZIER
                        if ("Q" == cmd.type)
                            for (var p = sampleQuadraticBezier(n[n.length - 1], {
                                x: cmd.x1,
                                y: cmd.y1
                            }, {
                                x: cmd.x,
                                y: cmd.y
                            }, t), d = 0; d < p.length; d++) h = appendToArrayOr(n, p[d]) ? h : h + 1;
                        //CLOSEPATH 
                        if ("Z" == cmd.type) {
                            var b = n[0],
                                g = n[n.length - 1];
                            b.z = 0, 999 == g.z && n.pop(), n.length - h > 2 && n.push(b), r.push(n), n = [], h = 0
                        }
                    }
                    return r
                }

                function markCornerPoints(points) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 145,
                        r = arguments.length > 2 ? arguments[2] : void 0;
                    point1 = {
                        x: points[0].x,
                        y: points[0].y,
                        z: 1
                    };
                    for (var n = 1; n < points.length - 1; n++) {
                        var h = {
                            x: points[n].x,
                            y: points[n].y,
                            z: points[n].z
                        },
                            a = {
                                x: points[n + 1].x,
                                y: points[n + 1].y,
                                z: points[n + 1].z
                            },
                            i = calculateAngleBetweenPoints_B([point1.x, point1.y], [h.x, h.y], [a.x, a.y]);
                        (r || 1 == points[n].z) && (points[n].z = i <= t && i > 0 ? 1 : 0), point1 = h
                    }
                    return points
                }

                function normalizeAndCenterLines(linesContainer, isHorizontalAdjustment, flipHorizontal) {
                    var n = linesContainer.lines,
                        h = linesContainer.w,
                        a = linesContainer.h,
                        i = {
                            left: 99999,
                            top: 99999,
                            right: -99999,
                            bottom: -99999,
                            width: 0,
                            height: 0,
                            x0: 0,
                            y0: 0
                        };
                    if (0 == n.length) i = {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                        width: 200,
                        height: 200,
                        x0: 0,
                        y0: 0
                    };
                    else {
                        for (var c = 0; c < n.length; c++)
                            for (var o = n[c], s = 0; s < o.length; s++) {
                                var l = [o[s].x, o[s].y];
                                i.left = Math.min(i.left, l[0]), i.top = Math.min(i.top, l[1]), i.right = Math.max(i.right, l[0]), i.bottom = Math.max(i.bottom, l[1])
                            }
                        i.width = i.right - i.left, i.height = i.bottom - i.top, i.x0 = i.left + i.width / 2, i.y0 = i.top + i.height / 2
                    }
                    for (var p = [], d = 0; d < n.length; d++) {
                        for (var b = n[d], g = [], j = 0; j < b.length; j++) {
                            var x = {
                                x: b[j].x,
                                y: b[j].y,
                                z: b[j].z
                            };
                            isHorizontalAdjustment ? x.x = flipHorizontal ? -x.x + 2 * i.x0 - i.left + 20 : x.x - i.left + 20 : x.y = x.y - i.top + 20, g.push(x)
                        }
                        p.push(g)
                    }
                    isHorizontalAdjustment ? h = i.width + 40 : a = i.height + 40;
                    var V = {
                        lines: p,
                        w: h,
                        h: a
                    };
                    return V
                }

                function layoutAndSimplifyShapes(shapes, markCorners, isHorizontalLayout, simplify, flipHorizontal) {
                    var totalWidth = 0, totalHeight = 0;
                    for (var shapeIndex = 0; shapeIndex < shapes.length; shapeIndex++) {
                        shapes[shapeIndex] = normalizeAndCenterLines(shapes[shapeIndex], isHorizontalLayout, flipHorizontal);
                        var normalizedShape = shapes[shapeIndex];
                        if (isHorizontalLayout) {
                            totalWidth += normalizedShape.w;
                            totalHeight = normalizedShape.h;
                        } else {
                            totalWidth = normalizedShape.w;
                            totalHeight += normalizedShape.h;
                        }
                    }
                    var result = [],
                        offsetX = -totalWidth / 2,
                        offsetY = totalHeight / 2,
                        layoutX = 0,
                        layoutY = 0;
                    for (var shapeIter = 0; shapeIter < shapes.length; shapeIter++) {
                        var shape = shapes[shapeIter],
                            lines = shape.lines;
                        if (!isHorizontalLayout) {
                            layoutX = -shape.w / 2;
                            offsetX = 0;
                        }
                        for (var lineIndex = 0; lineIndex < lines.length; lineIndex++) {
                            var line = lines[lineIndex],
                                simplifiedLine = [],
                                firstPoint = {
                                    x: offsetX + line[0].x + layoutX,
                                    y: offsetY - line[0].y + layoutY,
                                    z: 1
                                };
                            if (simplify) {
                                if (markCorners) {
                                    line = markCornerPoints(line, 135, false);
                                } else {
                                    var pointIdx = 1;
                                    while (pointIdx < line.length) {
                                        var currentPoint = {
                                            x: offsetX + line[pointIdx].x + layoutX,
                                            y: offsetY - line[pointIdx].y + layoutY,
                                            z: line[pointIdx].z
                                        };
                                        if (distanceBetweenPoints(firstPoint, currentPoint) < 2) {
                                            line.splice(pointIdx, 1);
                                        } else {
                                            pointIdx++;
                                            firstPoint = currentPoint;
                                        }
                                    }
                                    line = markCornerPoints(line, 145, true);
                                }
                            }
                            firstPoint = {
                                x: offsetX + line[0].x + layoutX,
                                y: offsetY - line[0].y + layoutY,
                                z: 1
                            };
                            simplifiedLine.push(firstPoint);
                            var midIdx = 1;
                            while (midIdx < line.length - 1) {
                                var midPoint = {
                                    x: offsetX + line[midIdx].x + layoutX,
                                    y: offsetY - line[midIdx].y + layoutY,
                                    z: line[midIdx].z
                                },
                                    nextPoint = {
                                        x: offsetX + line[midIdx + 1].x + layoutX,
                                        y: offsetY - line[midIdx + 1].y + layoutY,
                                        z: line[midIdx + 1].z
                                    };
                                if (simplify) {
                                    var angle = calculateAngleBetweenPoints_B([firstPoint.x, firstPoint.y], [midPoint.x, midPoint.y], [nextPoint.x, nextPoint.y]);
                                    if ((angle === 0 || angle > 174) && midPoint.z === 0) {
                                        line.splice(midIdx, 1);
                                        if (midIdx > 1) {
                                            midIdx--;
                                            simplifiedLine.pop();
                                            firstPoint = simplifiedLine[simplifiedLine.length - 1];
                                        }
                                        continue;
                                    }
                                    if (midPoint.z === 0 && distanceBetweenPoints(simplifiedLine[simplifiedLine.length - 1], midPoint) < 20) {
                                        line.splice(midIdx, 1);
                                        if (midIdx > 1) {
                                            midIdx--;
                                            simplifiedLine.pop();
                                            firstPoint = simplifiedLine[simplifiedLine.length - 1];
                                        }
                                        continue;
                                    }
                                }
                                simplifiedLine.push(midPoint);
                                firstPoint = midPoint;
                                midIdx++;
                            }
                            var lastPoint = {
                                x: offsetX + line[line.length - 1].x + layoutX,
                                y: offsetY - line[line.length - 1].y + layoutY,
                                z: 1
                            };
                            simplifiedLine.push(lastPoint);
                            result.push([shapeIter, simplifiedLine, shape.w, shape.h]);
                        }
                        if (lines.length === 0) {
                            var placeholder = [{
                                x: offsetX + shape.w / 2 + layoutX,
                                y: 0,
                                z: 0
                            }];
                            result.push([shapeIter, placeholder, shape.w, shape.h]);
                        }
                        if (isHorizontalLayout) {
                            layoutX += shape.w;
                        } else {
                            layoutY -= shape.h;
                        }
                    }
                    if (simplify && !markCorners) {
                        result = (function (linesArr) {
                            for (var arrIdx = 0; arrIdx < linesArr.length; arrIdx++) {
                                var lineArr = linesArr[arrIdx][1];
                                if (!(lineArr.length < 4)) {
                                    var startAngle = calculateAngleBetweenPoints_B([
                                        lineArr[lineArr.length - 2].x,
                                        lineArr[lineArr.length - 2].y
                                    ], [
                                        lineArr[0].x,
                                        lineArr[0].y
                                    ], [
                                        lineArr[1].x,
                                        lineArr[1].y
                                    ]);
                                    if (startAngle > 145 || startAngle === 0) {
                                        for (var cornerIdx = 1; cornerIdx < lineArr.length - 1; cornerIdx++) {
                                            var newArr = [];
                                            if (lineArr[cornerIdx].z === 1) {
                                                for (var i = cornerIdx; i < lineArr.length - 1; i++) newArr.push(lineArr[i]);
                                                for (var c = 0; c <= cornerIdx; c++) {
                                                    if (c === 0) lineArr[c].z = 0;
                                                    newArr.push(lineArr[c]);
                                                }
                                                if (newArr.length !== 0) {
                                                    linesArr[arrIdx][1] = newArr;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            return linesArr;
                        })(result);
                    }
                    return result;
                }

                function distanceBetweenPoints(pointA, pointB) {
                    var r = Math.pow(pointA.x - pointB.x, 2),
                        n = Math.pow(pointA.y - pointB.y, 2),
                        h = Math.sqrt(r + n);
                    return h
                }

                function markPolylineCorners(polylinePoints) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 145,
                        r = arguments.length > 2 ? arguments[2] : void 0;
                    point1 = {
                        x: polylinePoints[0][0],
                        y: polylinePoints[0][1],
                        z: 1
                    };
                    for (var n = 1; n < polylinePoints.length - 1; n++) {
                        var h = {
                            x: polylinePoints[n][0],
                            y: polylinePoints[n][1],
                            z: polylinePoints[n][3]
                        },
                            a = {
                                x: polylinePoints[n + 1][0],
                                y: polylinePoints[n + 1][1],
                                z: polylinePoints[n + 1][3]
                            },
                            i = calculateAngleBetweenPoints_B([point1.x, point1.y], [h.x, h.y], [a.x, a.y]);
                        (r || 1 == polylinePoints[n][3]) && (polylinePoints[n][3] = i <= t && i > 0 ? 1 : 0), point1 = h
                    }
                    return polylinePoints
                }

                function rotatePolylineToCornerStart(polyLinePoints) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 145;
                    if (polyLinePoints.length < 4) return polyLinePoints;
                    if (polyLinePoints[0][0] != polyLinePoints[polyLinePoints.length - 1][0] || polyLinePoints[0][1] != polyLinePoints[polyLinePoints.length - 1][1]) return polyLinePoints;
                    var r = calculateAngleBetweenPoints_B([polyLinePoints[polyLinePoints.length - 2][0], polyLinePoints[polyLinePoints.length - 2][1]], [polyLinePoints[0][0], polyLinePoints[0][1]], [polyLinePoints[1][0], polyLinePoints[1][1]]);
                    if (r > t || 0 == r)
                        for (var n = 1; n < polyLinePoints.length - 1; n++) {
                            var h = [];
                            if (1 == polyLinePoints[n][3]) {
                                for (var a = n; a < polyLinePoints.length - 1; a++) a == n ? h.push([polyLinePoints[a][0], polyLinePoints[a][1], 0, polyLinePoints[a][3]]) : h.push(polyLinePoints[a]);
                                for (var i = 0; i <= n; i++) 0 == i && (polyLinePoints[i][3] = 0, polyLinePoints[i][2] = polyLinePoints[i + 1][2]), h.push(polyLinePoints[i]);
                                if (0 != h.length) return h;
                                break
                            }
                        }
                    return polyLinePoints
                }

                function calculateAngleBetweenPoints_B(e, point, r) {
                    var n = {
                        x: e[0] - point[0],
                        y: e[1] - point[1]
                    },
                        h = {
                            x: r[0] - point[0],
                            y: r[1] - point[1]
                        },
                        dotProduct = n.x * h.x + n.y * h.y,
                        i = Math.sqrt(Math.pow(n.x, 2) + Math.pow(n.y, 2)),
                        c = Math.sqrt(Math.pow(h.x, 2) + Math.pow(h.y, 2));
                    if (0 == i || 0 == c) return 0;
                    var o = Math.acos(dotProduct / (i * c)),
                        s = 180 * o / Math.PI;
                    return s
                }

                function isArabic(e) {
                    return /[\u0600-\u06FF\uFE80-\uFEFF]/.test(e)
                }

                function searchArabic(e) {
                    if ("" == e) return !1;
                    for (var t = 0; t < e.length; t++)
                        if (isArabic(e[t])) return !0;
                    return !1
                }

                function reverseWithArabicSupport(input) {
                    for (var t = "", r = "", n = 0, h = 0; h < input.length; h++) {
                        var a = input[h];
                        isArabic(a) ? (0 == n && (t = r + t, r = ""), n = 1, r += a) : " " == a ? (t = t + r + a, r = "", n = 0) : (1 == n && (t = r + t, r = ""), n = 0, r = a + r)
                    }
                    return "" != r && (t += r), t = t.split("").reverse().join(""), t
                }

                function transformPolylinesForVerticalMirroring(inputPolylines, unused, width, height) {
                    for (var h = [], a = [], i = 0; i < inputPolylines.length; i++) {
                        for (var c = [], o = [], s = 0; s < inputPolylines[i].length; s++) {
                            var l = inputPolylines[i][s];
                            c.push({
                                x: l.y,
                                y: -l.x + width / 2 + .4 * height,
                                z: l.z
                            }), o.push({
                                x: -l.y,
                                y: -l.x + width / 2 + .4 * height,
                                z: l.z
                            })
                        }
                        h.push(c), a.push(o)
                    }
                    return {
                        newLinesUp: h,
                        newLinesDown: a
                    }
                }

                function getTextLines(loadedFontOpentype, text) {
                    var numberOfSegments = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : 5,
                        generateMirrorLines = arguments.length > 4 && void 0 !== arguments[4] && arguments[4];
                    try {
                        var fontSize = 400,
                            inputText = text,
                            hasArabic = searchArabic(inputText);
                        if (hasArabic) {
                            inputText = arabicHelper.convertArabic(inputText);
                            inputText = reverseWithArabicSupport(inputText);
                        }
                        var linesArr = [],
                            linesArrUp = [],
                            linesArrDown = [],
                            notRecognized = "";

                        // console.log(inputText);
                        for (var charIndex = 0; charIndex < inputText.length; charIndex++) {
                            var letter = inputText[charIndex],
                                glyph = loadedFontOpentype.charToGlyph(letter),
                                baseline = fontSize * loadedFontOpentype.ascender / (loadedFontOpentype.ascender - loadedFontOpentype.descender),
                                glyphPath = glyph.getPath(0, baseline, fontSize),
                                boundingBox = glyphPath.getBoundingBox(),
                                glyphHeight = Math.abs(boundingBox.y1) + Math.abs(boundingBox.y2),
                                glyphWidth = Math.abs(boundingBox.x1) + Math.abs(boundingBox.x2);
                            glyphWidth = glyphWidth === 0 ? fontSize / 2 : glyphWidth;
                            glyphHeight = glyphHeight === 0 ? fontSize : 1.1 * glyphHeight;
                            var polyline = [];
                            if (letter !== " " && (glyph.index !== 0 || glyph.unicodes.length !== 0)) {
                                var glyphCommands = glyphPath.commands;
                                polyline = parsePathCommands(glyphCommands, numberOfSegments);
                            }
                            if (polyline.length === 0) {
                                notRecognized += letter;
                            }
                            if (generateMirrorLines) {
                                var mirrored = transformPolylinesForVerticalMirroring(polyline, 0, glyphWidth, fontSize);
                                linesArrUp.push({
                                    lines: mirrored.newLinesUp,
                                    w: glyphWidth,
                                    h: glyphHeight
                                });
                                linesArrDown.push({
                                    lines: mirrored.newLinesDown,
                                    w: glyphWidth,
                                    h: glyphHeight
                                });
                            }
                            linesArr.push({
                                lines: polyline,
                                w: glyphWidth,
                                h: glyphHeight
                            });
                            //console.log({
                            //    lines: polyline,
                            //    w: glyphWidth,
                            //    h: glyphHeight
                            //});
                        }
                        return {
                            linesArr: linesArr,
                            linesArrUp: linesArrUp,
                            linesArrDown: linesArrDown,
                            notRec: notRecognized,
                            hasArb: hasArabic
                        };
                    } catch (error) {
                        console.log(error);
                    }
                }

                function normalizeAndCenterPolylines(polyLinePoints, targetWidth, r) {
                    for (var n = targetWidth / 800, h = [], a = [], i = 0, c = 0, o = 99999, s = 99999, index = 0; index < polyLinePoints.length; index++) {
                        var p = polyLinePoints[index],
                            d = [p[0] * n, p[1] * n, p[2], p[3]];
                        i < d[0] && (i = d[0]), c < d[1] && (c = d[1]), o > d[0] && (o = d[0]), s > d[1] && (s = d[1])
                    }
                    var b = -o,
                        g = i - o + .1 * targetWidth,
                        j = targetWidth;
                    r || (g = targetWidth, b = targetWidth / 2 - ((i - o) / 2 + o), j = c - s + .1 * targetWidth);
                    for (var x = 0; x < polyLinePoints.length; x++) {
                        var V = polyLinePoints[x],
                            f = [V[0] * n, V[1] * n, V[2], V[3]];
                        0 == f[2] && a.length > 0 && (h.push(a), a = []), a.push({
                            x: f[0] + b,
                            y: j / 2 - f[1],
                            z: f[3]
                        })
                    }
                    return a.length > 0 && h.push(a), {
                        lines: h,
                        w: g,
                        h: j
                    }
                }

                function getCharHexCode(e) {
                    var t = e[0],
                        r = t.charCodeAt(0),
                        n = r.toString(16);
                    return n.toLowerCase()
                }

                function unpackEncodedNumber(e) {
                    var t = e % 10,
                        r = function (e) {
                            var t, r, n;
                            return e < 4 ? (t = e, r = 0, n = 1) : e < 7 ? (t = e - 3, r = 1, n = 0) : (t = e - 6, r = 1, n = 1), [t, r, n]
                        }(t),
                        h = spreadToArrayHelper(r, 3),
                        a = h[0],
                        i = h[1],
                        c = h[2],
                        o = Math.pow(10, a + 1),
                        s = e % o,
                        l = Math.floor((e - s) / o - 400);
                    return s = Math.floor((s - t) / 10 - 400), [s, l, i, c]
                }
                var numericsAndAlphas = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+/";

                function base64CustomToInt(e) {
                    for (var t = 0, r = 0; r < e.length; r++) t = 64 * t + numericsAndAlphas.indexOf(e.charAt(r));
                    return t
                }

                function decodePackedNumberList(inputString) {
                    for (var t = [], r = function (e) {
                        for (var t = e.split(","), r = [], n = 0; n < t.length; n++) {
                            var h = base64CustomToInt(t[n]);
                            r.push(h)
                        }
                        return r
                    }(inputString), n = 0; n < r.length; n++) {
                        var h = unpackEncodedNumber(r[n]);
                        t.push(h)
                    }
                    return t
                }

                function mirrorPolylineFields(e) {
                    for (var t = [], r = [], h = 0; h < e.length; h++) {
                        var a = spreadToArrayHelper(e[h], 4),
                            i = a[0],
                            c = a[1],
                            o = a[2],
                            s = a[3];
                        t.push([-c, i, o, s]), r.push([c, i, o, s])
                    }
                    return {
                        xysUp: t,
                        xysDown: r
                    }
                }

                function getCharacterPolylineData(polyLinelookup, characters) {
                    for (var n = arguments.length > 2 && void 0 !== arguments[2] && arguments[2], h = !(arguments.length > 3 && void 0 !== arguments[3]) || arguments[3], a = arguments.length > 4 && void 0 !== arguments[4] && arguments[4], i = 400, c = [], o = [], s = [], l = "", p = 0; p < characters.length; p++) {
                        var d = getCharHexCode(characters[p]),
                            b = [],
                            g = [],
                            j = [],
                            x = i / 3,
                            V = i,
                            f = i,
                            F = i / 3;
                        if (d in polyLinelookup) {
                            var k = polyLinelookup[d];
                            if (n && (k = decodePackedNumberList(k)), a) {
                                t("log", "xysVer", JSON.stringify(k), " at utils/TextLine.js:680");
                                var u = JSON.parse(JSON.stringify(k));
                                u = mirrorPolylineFields(u);
                                var X = normalizeAndCenterPolylines(u.xysUp, i, h);
                                g = X.lines, f = X.w, F = X.h;
                                var N = normalizeAndCenterPolylines(u.xysDown, i, h);
                                j = N.lines
                            }
                            var Q = normalizeAndCenterPolylines(k, i, h);
                            b = Q.lines, x = Q.w, V = Q.h
                        } else l += characters[p];
                        c.push({
                            lines: b,
                            w: x,
                            h: V
                        }), o.push({
                            lines: g,
                            w: f,
                            h: F
                        }), s.push({
                            lines: j,
                            w: f,
                            h: F
                        })
                    }
                    return {
                        linesArr: c,
                        linesArrUp: o,
                        linesArrDown: s,
                        notRec: l
                    }
                }
                e.exports = {
                    getTextLines: getTextLines,
                    layoutAndSimplifyShapes: layoutAndSimplifyShapes,
                    getXXYY: function (opentype, font, inputText, mirrorVertical) {
                        console.log(font.mode);
                        var isHorizontalLayout = !(arguments.length > 4 && void 0 !== arguments[4]) || arguments[4],
                            numSegments = arguments.length > 5 && void 0 !== arguments[5] ? arguments[5] : 5,
                            textLines = {},
                            mainResult = [],
                            mirroredRightResult = [],
                            mirroredUpResult = [],
                            mirroredDownResult = [],
                            reversedLinesArr = [];
                        if (font.mode === 1) {
                            textLines = getTextLines(font.data, inputText, numSegments, mirrorVertical);
                            mainResult = layoutAndSimplifyShapes(textLines.linesArr, false, isHorizontalLayout, true, false);
                            mirroredUpResult = layoutAndSimplifyShapes(textLines.linesArrUp, false, isHorizontalLayout, true, false);
                            mirroredDownResult = layoutAndSimplifyShapes(textLines.linesArrDown, false, isHorizontalLayout, true, false);
                            reversedLinesArr = JSON.parse(JSON.stringify(textLines.linesArr));
                            reversedLinesArr.reverse();
                            mirroredRightResult = layoutAndSimplifyShapes(reversedLinesArr, false, isHorizontalLayout, true, true);
                        } else {
                            if (font.mode !== 2) return {
                                xxyy: [],
                                notRec: "",
                                XxyyRight: [],
                                xxyyUp: [],
                                xxyyDown: mirroredDownResult
                            };
                            textLines = getCharacterPolylineData(font.data, inputText, true, isHorizontalLayout, mirrorVertical);
                            mainResult = layoutAndSimplifyShapes(textLines.linesArr, true, isHorizontalLayout, true, false);
                            mirroredUpResult = layoutAndSimplifyShapes(textLines.linesArrUp, true, isHorizontalLayout, true, false);
                            mirroredDownResult = layoutAndSimplifyShapes(textLines.linesArrDown, true, isHorizontalLayout, true, false);
                            reversedLinesArr = JSON.parse(JSON.stringify(textLines.linesArr));
                            reversedLinesArr.reverse();
                            mirroredRightResult = layoutAndSimplifyShapes(reversedLinesArr, true, isHorizontalLayout, true, true);
                        }
                        return {
                            xxyy: mainResult,
                            notRec: textLines.notRec,
                            xxyyRight: mirroredRightResult,
                            xxyyUp: mirroredUpResult,
                            xxyyDown: mirroredDownResult
                        }
                    },
                    dealObjLines: function (polylinePoints) {
                        for (var filtering = !(arguments.length > 1 && void 0 !== arguments[1]) || arguments[1], r = 20, n = [], h = [], a = {
                            left: 99999,
                            top: -99999,
                            right: -99999,
                            bottom: 99999
                        }, i = polylinePoints, c = 0; c < i.length; c++) {
                            var o = [i[c][0], i[c][1]];
                            a.left = Math.min(a.left, o[0]), a.top = Math.max(a.top, o[1]), a.right = Math.max(a.right, o[0]), a.bottom = Math.min(a.bottom, o[1])
                        }
                        for (var s = (a.right - a.left) / 2 + a.left, l = (a.top - a.bottom) / 2 - a.top, p = 0; p < polylinePoints.length; p++) {
                            var d = polylinePoints[p],
                                b = d[3];
                            if (filtering && 0 != h.length && 0 != d[2] && p < polylinePoints.length - 1) {
                                var g = polylinePoints[p + 1];
                                if (0 != g[2]) {
                                    var x = calculateAngleBetweenPoints_B(h, d, g);
                                    if (0 == x || x >= 166) continue;
                                    if (b = x <= 145 ? 1 : 0, 0 == b && Math.abs(d[d.length - 1][0] - d[0]) + Math.abs(d[d.length - 1][1] - d[1]) < r) continue
                                }
                            }
                            n.push([d[0] + s, d[1] + l, d[2], b]), h = d
                        }
                        return n
                    },
                    dealImgLines: function (polylinePoints) {
                        for (var t = [], index = 0; index < polylinePoints.length; index++) {
                            var n = markPolylineCorners(polylinePoints[index], 135, !1);
                            n = rotatePolylineToCornerStart(n, 135), t.push.apply(t, arrayConversionHelper(n))
                        }
                        return t
                    },
                    printXXYY: function (e, r) {
                        for (var n = "", h = -1, a = 0; a < e.length; a++) {
                            var i = e[a][1];
                            h != e[a][0] && ("" != n && t("log", h, "printXXYY", n, " at utils/TextLine.js:713"), h = e[a][0], n = "");
                            for (var c = 0; c < i.length; c++) {
                                var o = i[c],
                                    s = (2 * o.x).toFixed(0) + " * 1",
                                    l = (2 * o.y).toFixed(0) + " * 1",
                                    p = r,
                                    d = o.z;
                                0 == c && (p = 0, d = 1), n = n + "\n{" + s.padStart(8, " ") + "," + l.padStart(8, " ") + ", " + p + ", " + d + "},"
                            }
                        }
                        t("log", h, "printXXYY", n, " at utils/TextLine.js:731")
                    }
                }
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "spreadToArrayHelper": function (e, t, r) {
            var n = r("arrayIfArrayHelper"),
                h = r("iterableToArrayLimitHelper"),
                a = r("toConsumableArrayHelper"),
                i = r("nonIterableDestructuringErrorHelper");
            e.exports = function (e, t) {
                return n(e) || h(e, t) || a(e, t) || i()
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },

        "arrayConversionHelper": function (e, t, r) {
            var n = r("arrayToArrayLikeHelper"),
                h = r("b893"),
                a = r("toConsumableArrayHelper"),
                i = r("nonIterableSpreadErrorHelper");
            e.exports = function (e) {
                return n(e) || h(e) || a(e) || i()
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },

        "handDrawPageComponent": function (e, t, r) {
            "use strict";
            (function (e) {
                var n = r("esModuleInteropHelper");
                Object.defineProperty(t, "__esModule", {
                    value: !0
                }), t.default = void 0;
                var h = n(r("uniPopupComponentExportWrapper")),
                    app = getApp(),
                    codePointAt = r("codePointAt"),
                    textLineVectorizer = r("textLineVectorizer "),
                    deviceCommandUtils = r("deviceCommandUtils "),
                    handDrawFileManager = r("handDrawFileManager"),
                    bleDeviceControlUtils = r("bleDeviceControlUtils "),
                    colors = ["black", "red", "green", "blue", "yellow", "#00FFFF", "purple", "white"],
                    fontGeometryUtils = r("fontGeometryUtils"),
                    handDrawGeometryUtils = r("handDrawGeometryUtils "),
                    g = [15, 5],
                    j = [20, 20],
                    x = {
                        data: function () {
                            var e = 0 | app.globalData.readData("draw_fontIdex"),
                                t = app.globalData.getDeviceFeatures(),
                                r = app.globalData.getTipsParm(),
                                n = handDrawFileManager.handDrawClassFix,
                                h = 650 * app.globalData.screen_width_float;
                            return {
                                screen_width: app.globalData.screen_width_str,
                                scUnit: app.globalData.screen_width_float,
                                rtl: app.globalData.rtl,
                                ntitle: this.$t("Hand-drawn doodle"),
                                inputText: "",
                                inputNote: this.$t("Please enter text"),
                                fontIdex: e,
                                fontNameList: [],
                                selectModeRange: [32, 32],
                                selectMode: !1,
                                selectClick: !1,
                                selectPoints: [],
                                selectLines: [],
                                selectRect: null,
                                selectDistance: null,
                                features: t,
                                showTips: r,
                                textToLeft: !0,
                                zoomObj: !1,
                                showSending: !1,
                                needReDraw: !1,
                                lineCtx: null,
                                showChineseWarn: !1,
                                lastCmdTime: 0,
                                lastSendtime: 0,
                                lastCompleteTime: 0,
                                showCanvas: !0,
                                drawMode: -1,
                                createTextPointsTimer: null,
                                drawTimerSub: null,
                                lineColor: 9,
                                subCanvasStartPoint: {
                                    x: 0,
                                    y: 0
                                },
                                subCanvasEndPoint: {
                                    x: 0,
                                    y: 0
                                },
                                lastPoint: {
                                    x: 0,
                                    y: 0
                                },
                                lastLinePts: [],
                                linePtsSendSn: 0,
                                sendLineMode: 0,
                                drawCanvas: {
                                    w: 0,
                                    h: 0
                                },
                                btnDrawGroup: {
                                    w: 0,
                                    h: 0,
                                    x: "true",
                                    y: "false",
                                    wrap: "nowrap"
                                },
                                points: [],
                                OutPts: [],
                                drawAddFileName: "",
                                currSelectedFile: "",
                                handDrawClass: n,
                                handDrawClassName: [],
                                handDrawClassIdx: 0,
                                lastRefresh: 0,
                                chPer: 1,
                                chBeginPoint: {
                                    x: 0,
                                    y: 0
                                },
                                chEndPoint: {
                                    x: 0,
                                    y: 0
                                },
                                chDraw: {
                                    w: 0,
                                    h: 0,
                                    max: 255
                                },
                                chCanvas: {
                                    w: 0,
                                    h: 0
                                },
                                cnfIdx: 4,
                                pisObj: {
                                    txPointTime: 50,
                                    cnfValus: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                                },
                                pisObjNote: {
                                    0: [
                                        [256, this.$t("Pattern Selection")]
                                    ],
                                    1: [
                                        [25, this.$t("Straight Line Pattern")],
                                        [25, this.$t("Arc Pattern")],
                                        [25, this.$t("Bright Spot Pattern")],
                                        [25, this.$t("Dot Pattern")],
                                        [25, this.$t("Christmas Pattern")],
                                        [25, this.$t("Animation Group 1")],
                                        [25, this.$t("Animation Group 2")],
                                        [25, this.$t("Animation Group 3")],
                                        [25, this.$t("Animation Group 4")],
                                        [31, this.$t("Animation Group 5")]
                                    ],
                                    2: [
                                        [10, this.$t("White")],
                                        [10, this.$t("Red")],
                                        [10, this.$t("Blue")],
                                        [10, this.$t("Pink")],
                                        [10, this.$t("Cyan")],
                                        [10, this.$t("Yellow")],
                                        [10, this.$t("Green")],
                                        [10, this.$t("Overall Color Change")],
                                        [13, this.$t("Rainbow Colors")],
                                        [18, this.$t("2-Segment Color")],
                                        [21, this.$t("3-Segment Color")],
                                        [18, this.$t("4-Segment Color")],
                                        [33, this.$t("8-Segment Color")],
                                        [36, this.$t("16-Segment Color")],
                                        [35, this.$t("32-Segment Color")],
                                        [2, this.$t("Color Gradient Drawing")]
                                    ],
                                    3: [
                                        [10, this.$t("Non-Flowing Water")],
                                        [118, this.$t("Forward Flowing Water")],
                                        [128, this.$t("Reverse Flowing Water")]
                                    ],
                                    4: [
                                        [256, this.$t("Pattern Size")]
                                    ],
                                    5: [
                                        [16, this.$t("Expand Manual Selection")],
                                        [40, this.$t("From Small to Large Expansion")],
                                        [40, this.$t("From Large to Small Expansion")],
                                        [40, this.$t("Large to Small Expansion")],
                                        [120, this.$t("Preview No Function")]
                                    ],
                                    6: [
                                        [128, this.$t("Rotation Angle")],
                                        [64, this.$t("Forward Rotation Speed")],
                                        [64, this.$t("Reverse Rotation Speed")]
                                    ],
                                    7: [
                                        [128, this.$t("Water Surface Rotation Position")],
                                        [128, this.$t("Water Surface Rotation Speed")]
                                    ],
                                    8: [
                                        [128, this.$t("Vertical Rotation Position")],
                                        [128, this.$t("Vertical Rotation Speed")]
                                    ],
                                    9: [
                                        [128, this.$t("Water Surface Rotation")],
                                        [128, this.$t("Water Surface Movement")]
                                    ],
                                    10: [
                                        [128, this.$t("Vertical Rotation Position")],
                                        [128, this.$t("Vertical Rotation Movement")]
                                    ],
                                    11: [
                                        [2, this.$t("Non-Flowing Water")],
                                        [31, this.$t("Flowing Water Width 1")],
                                        [32, this.$t("Flowing Water Width 2")],
                                        [32, this.$t("Flowing Water Width 3")],
                                        [32, this.$t("Flowing Water Width 4")],
                                        [32, this.$t("Flowing Water Width 5")],
                                        [32, this.$t("Flowing Water Width 6")],
                                        [32, this.$t("Flowing Water Width 7")],
                                        [31, this.$t("Flowing Water Width 8")]
                                    ]
                                },
                                cnfList: [{
                                    name: this.$t("Pattern Size"),
                                    value: 255,
                                    idx: 4
                                }, {
                                    name: this.$t("Pattern Expansion"),
                                    value: 255,
                                    idx: 5
                                }, {
                                    name: this.$t("Pattern Rotation"),
                                    value: 255,
                                    idx: 6
                                }, {
                                    name: this.$t("Water Surface Rotation"),
                                    value: 255,
                                    idx: 7
                                }, {
                                    name: this.$t("Vertical Rotation"),
                                    value: 255,
                                    idx: 8
                                }, {
                                    name: this.$t("Water Surface Movement"),
                                    value: 255,
                                    idx: 9
                                }, {
                                    name: this.$t("Vertical Movement"),
                                    value: 255,
                                    idx: 10
                                }, {
                                    name: this.$t("Flowing Water"),
                                    value: 255,
                                    idx: 11
                                }],
                                colorSeg: [{
                                    color: [1, 1, 1, 1, 1, 4, 4, 4, 4, 4, 2, 2, 2, 2, 5, 5, 5, 5, 5, 3, 3, 3, 3, 6, 6, 6, 6, 6, 7, 7, 7, 7],
                                    name: "Flowing Water (8 segments)"
                                }, {
                                    color: [1, 1, 1, 1, 1, 4, 4, 4, 4, 4, 2, 2, 2, 2, 5, 5, 5, 5, 5, 3, 3, 3, 3, 6, 6, 6, 6, 6, 7, 7, 7, 7],
                                    name: "Flowing Water (8 segments)"
                                }, {
                                    color: [3, 3, 3, 3, 3, 3, 3, 3, 7, 7, 7, 7, 7, 7, 7, 7, 2, 2, 2, 2, 2, 2, 2, 2, 7, 7, 7, 7, 7, 7, 7, 7],
                                    name: "Pick 3 (4 segments)"
                                }, {
                                    color: [4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 6, 6],
                                    name: "Pick 4 (4 segments)"
                                }, {
                                    color: [7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2],
                                    name: "White Green (8 segments)"
                                }, {
                                    color: [3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7],
                                    name: "White Blue (8 segments)"
                                }, {
                                    color: [7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1],
                                    name: "White Red (8 segments)"
                                }, {
                                    color: [4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5],
                                    name: "Green Yellow (8 segments)"
                                }, {
                                    color: [7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1],
                                    name: "White Red (16 segments)"
                                }, {
                                    color: [6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5],
                                    name: "Blue Green (16 segments)"
                                }, {
                                    color: [6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4],
                                    name: "Yellow Purple (16 segments)"
                                }, {
                                    color: [4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3],
                                    name: "Blue Yellow (16 segments)"
                                }],
                                segDisplayOrder: [{
                                    name: "Pick 3 (4 segments)",
                                    color: "color-seg-1",
                                    order: 0,
                                    idx: 11
                                }, {
                                    name: "Pick 4 (4 segments)",
                                    color: "color-seg-2",
                                    order: 1,
                                    idx: 12
                                }, {
                                    name: "White Green (8 segments)",
                                    color: "color-seg-3",
                                    order: 2,
                                    idx: 13
                                }, {
                                    name: "White Blue (8 segments)",
                                    color: "color-seg-4",
                                    order: 3,
                                    idx: 14
                                }, {
                                    name: "White Red (8 segments)",
                                    color: "color-seg-5",
                                    order: 4,
                                    idx: 15
                                }, {
                                    name: "Green Yellow (8 segments)",
                                    color: "color-seg-6",
                                    order: 5,
                                    idx: 16
                                }, {
                                    name: "White Red (16 segments)",
                                    color: "color-seg-7",
                                    order: 6,
                                    idx: 17
                                }, {
                                    name: "Blue Purple (16 segments)",
                                    color: "color-seg-8",
                                    order: 7,
                                    idx: 18
                                }, {
                                    name: "Yellow Purple (16 segments)",
                                    color: "color-seg-9",
                                    order: 8,
                                    idx: 19
                                }, {
                                    name: "Blue Yellow (16 segments)",
                                    color: "color-seg-10",
                                    order: 9,
                                    idx: 20
                                }],
                                colorDisplayOrder: [{
                                    name: "Red",
                                    color: "red",
                                    order: 0,
                                    idx: 1
                                }, {
                                    name: "Yellow",
                                    color: "yellow",
                                    order: 1,
                                    idx: 4
                                }, {
                                    name: "Green",
                                    color: "green",
                                    order: 2,
                                    idx: 2
                                }, {
                                    name: "Blue",
                                    color: "#00FFFF",
                                    order: 3,
                                    idx: 5
                                }, {
                                    name: "Purple",
                                    color: "blue",
                                    order: 4,
                                    idx: 3
                                }, {
                                    name: "Blue",
                                    color: "purple",
                                    order: 5,
                                    idx: 6
                                }, {
                                    name: "White",
                                    color: "white",
                                    order: 6,
                                    idx: 7
                                }, {
                                    name: "Transparent",
                                    color: "transparent",
                                    order: 7,
                                    idx: 8
                                }, {
                                    name: "Default",
                                    color: "transparent",
                                    order: 8,
                                    idx: 9
                                }, {
                                    name: "Gradient",
                                    color: "transparent",
                                    order: 9,
                                    idx: 10
                                }],
                                MaxDrawPointCount: 800,
                                drawPointCount: 0,

                                //  drawMode: Number,
                                //  ps: Array,        // Array of points or grouped points
                                //  x0: Number,       // X origin
                                //  y0: Number,       // Y origin
                                //  z: Number,        // Scale
                                //  ang: Number,      // Rotation angle
                                //  lineColor: Number // Color index

                                drawPoints: [],
                                drawPointsHis: [],
                                objParm: {
                                    x0: 0,
                                    y0: 0,
                                    z: 1,
                                    ang: 0,
                                    ps: null,
                                    lineColor: 0
                                },
                                obj: [
                                    [0, 200, 0, 1],
                                    [300, -200, 2, 1],
                                    [-300, -200, 2, 1],
                                    [0, 200, 2, 1]
                                ],
                                objCount: 0,
                                xxyy: [],
                                position: {
                                    x: h,
                                    y: h
                                },
                                startPosition: {
                                    x: 0,
                                    y: 0
                                }
                            }
                        },
                        onLoad: function () {
                            var picArrayShapes = r("picArrayShapes");
                            this.objCount = picArrayShapes.picArray.length;
                            var fontRegistryModule = r("fontRegistryModule ");
                            this.fontNameList = fontRegistryModule.getFontNameList();
                            var drawData = app.globalData.getCmdData("drawData");
                            this.pisObj = drawData.pisObj, handDrawFileManager.clearDrawPointsHis()
                        },
                        onUnload: function () {
                            e("log", "onunload", " at sub/pages/draw/draw.js:203"), this.saveDeskTop()
                        },
                        onReady: function () {
                            var e = this;
                            setTimeout((function () {
                                e.initShow()
                            }), 20)
                        },
                        onHide: function () {
                            this.saveDeskTop()
                        },
                        onShow: function () {
                            var e = this;
                            this.needReDraw && (this.needReDraw = !1, setTimeout((function () {
                                e.reDraw(e.drawPoints)
                            }), 10))
                        },
                        components: {
                            uniPopup: h.default
                        },
                        computed: {
                            inputTextX: {
                                get: function () {
                                    return this.inputText
                                },
                                set: function (e) {
                                    var t = e.replace("\n", "");
                                    this.inputText = t, this.resetSelectMode()
                                }
                            }
                        },
                        methods: {
                            sendCmd: function () {
                                var e = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : "00",
                                    t = (new Date).getTime(),
                                    points = [];
                                "00" == e && (points = this.reDraw(this.drawPoints));
                                var drawCommand = deviceCommandUtils.getDrawCmdStr(points, this.pisObj, this.features, e),
                                    h = bleDeviceControlUtils.gosend("00" == e, drawCommand, this.sendComplete);
                                return h && (this.lastSendtime = t), h
                            },
                            sendLastCmd: function () {
                                var e = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : "00",
                                    t = this;
                                t.lastCmdTime <= t.lastSendtime || setTimeout((function () {
                                    t.lastCmdTime <= t.lastSendtime || (t.sendCmd(e), t.lastCmdTime > t.lastSendtime && t.sendLastCmd(e))
                                }), 10)
                            },
                            initShow: function () {
                                var e = this,
                                    t = uni.createSelectorQuery().in(e);
                                uni.showLoading({
                                    mask: !0
                                }), t.select("#drawCanvasContainer0").boundingClientRect((function (t) {
                                    e.drawCanvas.w = t.height, e.$set(e.drawCanvas, "h", t.height), setTimeout((function () {
                                        var t = uni.createSelectorQuery().in(e);
                                        t.select("#btn_draw_group").boundingClientRect((function (t) {
                                            e.setBtnDrawGroup(t.width, t.height), setTimeout((function () {
                                                e.readFontBase64(), e.setCanvasSub(), e.restoreDeskTop()
                                            }), 1e3)
                                        })).exec()
                                    }), 100)
                                })).exec()
                            },
                            inputBlur: function (e) {
                                this.createTextPoints(), this.drawMode = 9999
                            },
                            refreshTextPoints: function () {
                                var e = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : 1500;
                                null != this.createTextPointsTimer && (clearTimeout(this.createTextPointsTimer), this.createTextPointsTimer);
                                var t = this;
                                this.createTextPointsTimer = setTimeout((function () {
                                    t.createTextPointsTimer, t.createTextPoints(), t.drawMode = 9999
                                }), e)
                            },
                            setBtnDrawGroup: function (e, t) {
                                var r = 2 * this.scUnit;
                                t >= 90 * r ? (this.$set(this.btnDrawGroup, "h", t), this.$set(this.btnDrawGroup, "w", e), this.$set(this.btnDrawGroup, "x", "hidden"), this.$set(this.btnDrawGroup, "y", "auto"), this.$set(this.btnDrawGroup, "wrap", "wrap")) : (t = t < 60 * r ? 50 * r : t, this.$set(this.btnDrawGroup, "x", "auto"), this.$set(this.btnDrawGroup, "y", "hidden"), this.$set(this.btnDrawGroup, "wrap", "nowrap"), this.$set(this.btnDrawGroup, "h", t), this.$set(this.btnDrawGroup, "w", e))
                            },
                            createTextPoints: function () {
                                if ("" == this.inputText) this.xxyy = [];
                                else {
                                    uni.showLoading({
                                        title: this.$t("\u6b63\u5728\u751f\u6210\u5750\u6807\u70b9..."),
                                        mask: !0
                                    });
                                    var e = textLineVectorizer.getXXYY(codePointAt, fontGeometryUtils.fontData, this.inputText, !1, this.textToLeft);
                                    uni.hideLoading(), this.xxyy = e.xxyy, fontGeometryUtils.ifHasChinese(e.notRec) && 1001 == fontGeometryUtils.fontData.sn && app.globalData.showModalTips(this.$t("\u56e0\u5bb9\u91cf\u9650\u5236\uff0c\u90e8\u5206\u6c49\u5b57\u672a\u7eb3\u5165\u5b57\u5e93\uff0c\u5b8c\u6574\u5b57\u5e93\u8bf7\u524d\u5f80APP\u7248\u672c"), !0)
                                }
                            },
                            readFontBase64: function () {
                                var e = this,
                                    t = r("fontRegistryModule "),
                                    n = t.getFontList(this),
                                    h = n[this.fontIdex].file,
                                    a = n[this.fontIdex].mode,
                                    i = n[this.fontIdex].sn;
                                n = null, uni.showLoading({
                                    mask: !0,
                                    title: this.$t("\u6b63\u5728\u8bfb\u53d6\u5b57\u4f53...")
                                }), fontGeometryUtils.readTTF(h, a, (function (t, r) {
                                    fontGeometryUtils.fontData = {
                                        data: t,
                                        mode: r,
                                        sn: i
                                    }, e.createTextPoints(), uni.hideLoading()
                                }))
                            },
                            onFontChange: function (e) {
                                this.setFontIdex(e.detail.value)
                            },
                            setFontIdex: function (e) {
                                this.fontIdex != e && (this.fontIdex = e, app.globalData.saveData("draw_fontIdex", this.fontIdex), this.readFontBase64(), this.drawMode = 9999)
                            },
                            fontSelect: function (t) {
                                var r = this;
                                uni.navigateTo({
                                    url: "/sub/pages/font/font?fontIdex=" + this.fontIdex,
                                    events: {
                                        acceptDataFromOpenedPage: function (t) {
                                            e("log", "acceptDataFromOpenedPage", t, " at sub/pages/draw/draw.js:390"), r.setFontIdex(t.fontIdex)
                                        }
                                    }
                                })
                            },
                            resetSelectMode: function () {
                                return !!this.selectMode && (this.selectClick = !1, this.selectMode = !1, this.selectLines = [], this.selectRect = null, this.setCanvasSub(),
                                    this.reDraw(this.drawPoints), !0)
                            },
                            btnDrawChange: function (e) {
                                this.resetSelectMode();
                                var tag = e.currentTarget.dataset.tag;
                                if (9999 == tag
                                    && this.textToLeft
                                    && (this.textToLeft = !1, this.refreshTextPoints(0)),
                                    9998 == tag
                                    && (this.textToLeft || (this.textToLeft = !0, this.refreshTextPoints(0)), tag = 9999),
                                    tag >= 0 && tag < 9999) {
                                    var picArrayShapes = r("picArrayShapes"),
                                        picArray = picArrayShapes.picArray;
                                    this.obj = textLineVectorizer.dealObjLines(picArray[tag], !1)
                                }
                                this.drawMode = tag
                            },
                            getCurrentPointCount: function () {
                                return -1 == this.drawMode ? handDrawGeometryUtils.getPointCount(this.drawMode, this.points, !0) : 9999 == this.drawMode ? handDrawGeometryUtils.getPointCount(this.drawMode, this.xxyy) : handDrawGeometryUtils.getPointCount(this.drawMode, this.obj)
                            },
                            //                          1: Freehand drawing mode (user draws lines or shapes manually)
                            //9999: Text drawing mode (drawing vectorized text or handwriting)
                            //9998: Text drawing mode, right-to-left (used for RTL text, toggled in btnDrawChange)
                            //8888, 8887: Special selection or editing modes (used for selection rectangles or editing, seen in dealTouchEnd)
                            //0 and positive integers less than 9999: Predefined geometric shapes or patterns (selected from picArrayShapes, e.g., rectangles, circles, custom shapes)
                            //2: Likely a specific shape or object drawing mode (used in drawObj)
                            dealTouchEnd: function () {
                                if (-1 == this.drawMode) this.points = fontGeometryUtils.dealLine(this.points, !1);
                                else if (Math.abs(this.subCanvasStartPoint.x - this.subCanvasEndPoint.x) < 20 && 8888 != this.drawMode && Math.abs(this.subCanvasStartPoint.y - this.subCanvasEndPoint.y) < 20) return !1;
                                8888 == this.drawMode && (this.drawMode = 8887);
                                var e = this.getCurrentPointCount();
                                if (this.drawPointCount + e > 800) return app.globalData.showModalTips(this.$t("\u8d85\u51fa\u6700\u5927\u70b9\u6570") + 800 + "\uff0c" + this.$t("\u8d85\u51fa\u90e8\u5206\u5c06\u4e22\u5931"), !1), !1;
                                if (this.drawPointCount = this.drawPointCount + e, -1 == this.drawMode) return this.touchEnd(null), !0;
                                this.addToHis();
                                var canvasContext = uni.createCanvasContext("drawCanvas", this),
                                    drawConfig = (this.objParm.x0, this.objParm.y0, this.objParm.z, this.objParm.ang, this.lineColor, {
                                        ctx: canvasContext,
                                        w: this.drawCanvas.w,
                                        h: this.drawCanvas.h,
                                        draw_line_type: j,
                                        colorSeg: this.colorSeg
                                    });
                                return this.objParm.lineColor = this.lineColor,
                                    9999 == this.drawMode
                                        ? (this.objParm.drawMode = 9999, this.objParm.ps = this.xxyy,
                                            handDrawGeometryUtils.drawText(drawConfig, this.objParm),
                                            this.drawPoints.push(this.objParm))
                                        : (this.objParm.drawMode = 2, this.objParm.ps = this.obj,
                                            handDrawGeometryUtils.drawObj(drawConfig, this.objParm),
                                            this.drawPoints.push(this.objParm)), canvasContext.draw(!0), !0
                            },
                            addToHis: function () {
                                if (!(this.drawPoints.length <= 0)) {
                                    for (var drawPointsHistory = [], index = 0; index < this.drawPoints.length; index++) {
                                        var drawPoint = this.drawPoints[index];
                                        drawPointsHistory.push({
                                            drawMode: drawPoint.drawMode,
                                            ps: drawPoint.ps,
                                            x0: drawPoint.x0,
                                            y0: drawPoint.y0,
                                            z: drawPoint.z,
                                            ang: drawPoint.ang,
                                            lineColor: drawPoint.lineColor
                                        })
                                    }
                                    handDrawFileManager.pushDrawPointsHis(drawPointsHistory)
                                }
                            },
                            setCanvasSub: function () {
                                var t = arguments.length > 0 && void 0 !== arguments[0] ? arguments[0] : null;
                                e("log", "setCanvasSub", " at sub/pages/draw/draw.js:501");
                                var r = !1;
                                t || (t = uni.createCanvasContext("drawCanvasSub", this), t.clearRect(0, 0, this.drawCanvas.w, this.drawCanvas.h), r = !0), t.setFillStyle("white"), t.setFontSize(10), t.fillText(this.drawPointCount + "/800", .01 * this.drawCanvas.w, .99 * this.drawCanvas.h);
                                var n = 16;
                                t.setFontSize(n), t.setLineDash(g);
                                var h = this.drawCanvas.w - 12,
                                    a = this.drawCanvas.h - 12,
                                    i = 10;
                                t.moveTo(h + i, a), t.arc(h, a, i, 0, 2 * Math.PI);
                                var c = this.$t("\u7f16\u8f91") + " ",
                                    o = t.measureText(c).width;
                                Number.isNaN(o) && (o = t.measureText(c).width), this.selectModeRange[0] = this.drawCanvas.w - (h - i - o + i), t.fillText(c, h - i - o, (2 * i - n) / 2 + a - i + n), t.setStrokeStyle("white"), t.stroke(), this.selectMode && null == this.selectRect && (t.beginPath(), t.arc(h, a, i - 3, 0, 2 * Math.PI), t.setFillStyle("yellow"), t.fill()), r && t.draw()
                            },
                            drawSelectRect: function () {
                                var e = null,
                                    t = null != this.selectRect ? this.selectRect : this.getRect(this.drawPoints);
                                if (this.selectRect = t, null != t) {
                                    "ang" in this.selectRect == 0 && (this.selectRect["ang"] = 0, this.selectRect["startAng"] = 0, this.selectRect["lastAng"] = 0);
                                    var r = handDrawGeometryUtils.getSelectRectInfo(t);
                                    this.selectRect["x0"] = r.x0, this.selectRect["y0"] = r.y0, e = uni.createCanvasContext("drawCanvasSub", this), e.clearRect(0, 0, this.drawCanvas.w, this.drawCanvas.h);
                                    var n = t.lastAng - t.startAng + t.ang,
                                        h = handDrawGeometryUtils.getCenterCorss(n, r.x0, r.y0, 20);
                                    e.beginPath(), e.setLineDash(g), e.setStrokeStyle("#51D1EA"), e.setLineWidth(1), e.moveTo(r.p1.x, r.p1.y), e.lineTo(r.p2.x, r.p2.y), e.lineTo(r.p3.x, r.p3.y), e.lineTo(r.p4.x, r.p4.y), e.lineTo(r.p1.x, r.p1.y), e.stroke(), e.beginPath(), e.setLineDash([]), e.setStrokeStyle("yellow"), e.setLineWidth(1), e.moveTo(h.p1.x, h.p1.y), e.lineTo(h.p2.x, h.p2.y), e.moveTo(h.p11.x, h.p11.y), e.lineTo(h.p1.x, h.p1.y), e.lineTo(h.p12.x, h.p12.y), e.moveTo(h.p21.x, h.p21.y), e.lineTo(h.p2.x, h.p2.y), e.lineTo(h.p22.x, h.p22.y), e.moveTo(h.p3.x, h.p3.y), e.lineTo(h.p4.x, h.p4.y), e.moveTo(h.p31.x, h.p31.y), e.lineTo(h.p3.x, h.p3.y), e.lineTo(h.p32.x, h.p32.y), e.moveTo(h.p41.x, h.p41.y), e.lineTo(h.p4.x, h.p4.y), e.lineTo(h.p42.x, h.p42.y), e.stroke(), e.beginPath()
                                }
                                this.setCanvasSub(e), null != e && e.draw()
                            },
                            touchEndSub: function (e) {
                                var t = this;
                                null != this.drawTimerSub && (clearInterval(this.drawTimerSub), this.drawTimerSub = null);
                                var r = this;
                                if (!this.selectClick && !this.selectMode && this.drawMode >= -1 && this.dealTouchEnd() && this.setDrawPointsSelect(!0), !this.selectClick && this.selectMode) {
                                    var n = this.selectRect;
                                    null != n && Math.abs(this.subCanvasStartPoint.x - this.subCanvasEndPoint.x) < 4 && Math.abs(this.subCanvasStartPoint.y - this.subCanvasEndPoint.y) < 4 && (this.subCanvasStartPoint.x < n.left + n.mx || this.subCanvasStartPoint.x > n.left + n.mx + n.width * n.z || this.subCanvasStartPoint.y < n.top + n.my || this.subCanvasStartPoint.y > n.top + n.my + n.height * n.z) && this.resetSelectMode()
                                }
                                setTimeout((function () {
                                    if (r.selectMode) {
                                        r.drawSelectRect();
                                        var e = t.selectRect;
                                        null == e || 0 == e.mx && 0 == e.my && 1 == e.z && e.lastAng - e.startAng == 0
                                            || r.addToHis(), r.reDraw(r.drawPoints)
                                    } else r.setCanvasSub()
                                }), 1)
                            },
                            touchMoveSub: function (e) {
                                if (!this.selectClick) {
                                    var t = e.touches[0],
                                        r = t.x,
                                        n = t.y;
                                    if (-1 != this.drawMode || this.selectMode) {
                                        var h = this.drawCanvas;
                                        r = r < 0 ? 0 : r, r = r > h.w ? h.w : r, n = n < 0 ? 0 : n, n = n > h.h ? h.h : n;
                                        var a = r - this.subCanvasEndPoint.x,
                                            i = n - this.subCanvasEndPoint.y;
                                        if (!(Math.abs(a) < 5 && Math.abs(i) < 5)) {
                                            this.subCanvasEndPoint.x = r, this.subCanvasEndPoint.y = n;
                                            var c = {
                                                w: this.drawCanvas.w - 8,
                                                h: this.drawCanvas.h - 8
                                            },
                                                o = 0,
                                                l = 0;
                                            if (this.selectMode && null != this.selectRect) {
                                                var p = this.selectRect.z,
                                                    d = this.selectRect.mx,
                                                    g = this.selectRect.my,
                                                    j = handDrawGeometryUtils.getSelectRectInfo(this.selectRect, 0),
                                                    x = handDrawGeometryUtils.getUiRectSize(j),
                                                    V = 1;
                                                if (2 === e.touches.length) {
                                                    var f = handDrawFileManager.getDistance(e.touches);
                                                    V = f / this.selectDistance, this.selectDistance = f;
                                                    var F = V,
                                                        k = V;
                                                    x.width * V > c.w && (F = c.w / x.width), x.height * V > c.h && (k = c.h / x.height), V = Math.min(F, k), a = this.selectRect.width * p / 2 * (1 - V), i = this.selectRect.height * p / 2 * (1 - V)
                                                } else {
                                                    if (this.zoomObj) return;
                                                    if (this.selectRect.rotate) {
                                                        var m = handDrawGeometryUtils.calcAngle(this.selectRect.x0, this.selectRect.y0, r, n),
                                                            P = this.selectRect.lastAng;
                                                        this.selectRect.lastAng = m;
                                                        var u = handDrawGeometryUtils.getSelectRectInfo(this.selectRect, 0),
                                                            X = handDrawGeometryUtils.getUiRectSize(u);
                                                        this.selectRect.lastAng = P;
                                                        if (X.left < 4 && (o = 4 - X.left), X.left + X.width > this.drawCanvas.w - 4) {
                                                            if (0 != o) return;
                                                            o = this.drawCanvas.w - 4 - (X.left + X.width)
                                                        }
                                                        if (X.top < 4 && (l = 4 - X.top), X.top + X.height > this.drawCanvas.h - 4) {
                                                            if (0 != l) return;
                                                            l = this.drawCanvas.h - 4 - (X.top + X.height)
                                                        }
                                                        this.selectRect.lastAng = m, a = 0, i = 0
                                                    }
                                                }
                                                var N = x.left + a,
                                                    H = x.top + i;
                                                N = Math.max(N, 4), H = Math.max(H, 4);
                                                var z = x.width * V,
                                                    Q = x.height * V,
                                                    R = N + z,
                                                    v = H + Q;
                                                N = Math.min(this.drawCanvas.w - 4, R) - z, H = Math.min(this.drawCanvas.h - 4, v) - Q, d = N - x.left + d + o, g = H - x.top + g + l;
                                                var I = this.selectRect.lastAng - this.selectRect.startAng;
                                                this.selectRect.mx == d && this.selectRect.my == g && 1 == V && 0 == I || (this.selectRect.mx = d, this.selectRect.my = g, this.selectRect.z = p * V, this.drawSelectRect())
                                            }
                                        }
                                    } else this.touchMove(e)
                                }
                            },
                            touchStartSub: function (e) {
                                var t = this;
                                null != this.drawTimerSub && (clearInterval(this.drawTimerSub), this.drawTimerSub = null);
                                var r = this,
                                    n = e.touches[0],
                                    h = n.x,
                                    a = n.y;
                                if (this.subCanvasStartPoint.x = h, this.subCanvasStartPoint.y = a, this.subCanvasEndPoint.x = h, this.subCanvasEndPoint.y = a, this.zoomObj = !1, 2 === e.touches.length && (this.selectDistance = handDrawFileManager.getDistance(e.touches), this.zoomObj = !0), this.selectClick = !1, this.drawCanvas.w - this.selectModeRange[0] < h && this.drawCanvas.h - this.selectModeRange[1] < a) return this.selectClick = !0, this.selectDistance = null, null != this.selectRect ? this.selectMode = !0 : this.selectMode = !this.selectMode, this.selectLines = [], this.selectRect = null, void (this.selectMode && this.tipOpen());
                                if (this.selectMode) {
                                    if (null != this.selectRect) {
                                        this.selectRect["rotate"] = !1;
                                        var i = handDrawGeometryUtils.getSelectRectInfo(this.selectRect);
                                        this.selectRect["x0"] = i.x0, this.selectRect["y0"] = i.y0, this.selectRect.ang = this.selectRect.ang + this.selectRect.lastAng - this.selectRect.startAng, this.selectRect.startAng = 0, this.selectRect.lastAng = 0;
                                        if (Math.abs(h - i.x0) < 30 && Math.abs(a - i.y0) < 30) return;
                                        return this.selectRect["rotate"] = !0, this.selectRect.startAng = handDrawGeometryUtils.calcAngle(this.selectRect.x0, this.selectRect.y0, h, a), void (this.selectRect.lastAng = this.selectRect.startAng)
                                    }
                                    return this.selectPoints = [
                                        [h, a]
                                    ], this.selectLines = [], void (this.drawTimerSub = setInterval((function () {
                                        if (null != t.selectPoints) {
                                            var e = r.selectPoints[r.selectPoints.length - 1];
                                            if (!(Math.abs(e[0] - r.subCanvasEndPoint.x) < 4 && Math.abs(e[1] - r.subCanvasEndPoint.y) < 4)) {
                                                r.selectPoints.push([r.subCanvasEndPoint.x, r.subCanvasEndPoint.y]);
                                                var n = uni.createCanvasContext("drawCanvasSub", r);
                                                n.setStrokeStyle("yellow"), n.setLineDash([]);
                                                for (var h = r.selectPoints.length - 2; h < r.selectPoints.length; h++) 0 == h ? n.moveTo(r.selectPoints[h][0], r.selectPoints[h][1]) : n.lineTo(r.selectPoints[h][0], r.selectPoints[h][1]);
                                                n.stroke(), n.draw(!0), r.checkObjectSelect(r.drawPoints)
                                            }
                                        }
                                    }), 20))
                                }
                                this.drawMode < -1 || (-1 != this.drawMode ? this.drawTimerSub = setInterval((function () {
                                    var e = r.subCanvasEndPoint.x - r.subCanvasStartPoint.x,
                                        n = r.subCanvasEndPoint.y - r.subCanvasStartPoint.y;
                                    if (!(Math.abs(e) + Math.abs(n) < 15)) {
                                        var h = {
                                            w: 800,
                                            h: 800
                                        };
                                        9999 == r.drawMode && (h = handDrawGeometryUtils.getTextLineSize(r.xxyy, r.textToLeft));
                                        var a = r.subCanvasStartPoint.x + e / 2,
                                            i = r.subCanvasStartPoint.y + n / 2,
                                            c = Math.abs(e / h.w),
                                            o = Math.abs(n / h.h),
                                            s = c > o ? o : c;
                                        r.objParm = {
                                            x0: a,
                                            y0: i,
                                            z: s,
                                            ang: 0,
                                            ps: null,
                                            lineColor: r.lineColor
                                        };
                                        var l = uni.createCanvasContext("drawCanvasSub", r),
                                            p = {
                                                ctx: l,
                                                w: t.drawCanvas.w,
                                                h: t.drawCanvas.h,
                                                draw_line_type: j,
                                                colorSeg: t.colorSeg
                                            };
                                        l.clearRect(0, 0, r.drawCanvas.w, r.drawCanvas.h), 9999 == r.drawMode ? (r.objParm.ps = r.xxyy, handDrawGeometryUtils.drawText(p, r.objParm)) : (r.objParm.ps = r.obj, handDrawGeometryUtils.drawObj(p, r.objParm)), l.closePath(), l.beginPath(), l.setLineDash(g), l.setStrokeStyle("#51D1EA"), l.setLineWidth(1);
                                        var d = r.subCanvasStartPoint.x < r.subCanvasEndPoint.x ? r.subCanvasStartPoint.x : r.subCanvasEndPoint.x,
                                            x = r.subCanvasStartPoint.y < r.subCanvasEndPoint.y ? r.subCanvasStartPoint.y : r.subCanvasEndPoint.y;
                                        l.strokeRect(d, x, Math.abs(e), Math.abs(n)), l.stroke(), t.setCanvasSub(l), l.draw()
                                    }
                                }), 50) : this.touchStart(e))
                            },
                            checkObjectSelect: function (e) {
                                if (0 != e.length && 0 != this.selectPoints.length)
                                    for (var t = [this.selectPoints[this.selectPoints.length - 2], this.selectPoints[this.selectPoints.length - 1]], r = 0; r < e.length; r++)
                                        if (this.selectLines.length < r + 1 && this.selectLines.push({
                                            sel: !1,
                                            mx0: 0,
                                            my0: 0,
                                            color: null
                                        }), !this.selectLines[r].sel) {
                                            var n = e[r],
                                                h = n.x0,
                                                a = n.y0,
                                                i = n.z,
                                                c = n.ps,
                                                o = n.ang;
                                            if (-1 == n.drawMode) {
                                                var s = handDrawGeometryUtils.calcLinesAngXY(c, o);
                                                this.selectLines[r].sel = handDrawGeometryUtils.checkLine(s, h, a, i, t)
                                            } else if (9999 == n.drawMode) {
                                                var l = handDrawGeometryUtils.calcTextAngXY(n);
                                                this.selectLines[r].sel = handDrawGeometryUtils.checkText(l, h, a, i, t)
                                            } else {
                                                var p = handDrawGeometryUtils.calcObjAngXY(c, o);
                                                this.selectLines[r].sel = handDrawGeometryUtils.checkObj(p, h, a, i, t)
                                            }
                                        }
                            },
                            setDrawPointsSelect: function () {
                                var e = arguments.length > 0 && void 0 !== arguments[0] && arguments[0],
                                    t = this.drawPoints;
                                if (0 != t.length) {
                                    this.selectRect = null, this.selectLines = [];
                                    for (var r = 0; r < t.length; r++) this.selectLines.length < r + 1 && this.selectLines.push({
                                        sel: !1,
                                        mx0: 0,
                                        my0: 0,
                                        color: null
                                    }), e ? r == t.length - 1 && (this.selectLines[r].sel = !0) : this.selectLines[r].sel = !0;
                                    this.selectMode = !0
                                }
                            },
                            getRect: function (e) {
                                if (this.selectDistance = null, 0 == e.length) return null;
                                if (this.selectLines.length != e.length) return null;
                                for (var t = !1, r = {
                                    left: 99999,
                                    top: 99999,
                                    right: -99999,
                                    bottom: -99999
                                }, n = 0; n < e.length; n++)
                                    if (this.selectLines[n].sel) {
                                        t = !0;
                                        var h = e[n];
                                        r = -1 == h.drawMode ? handDrawGeometryUtils.getLineRect(h, r) : 9999 == h.drawMode ? handDrawGeometryUtils.getTextRect(h, r) : handDrawGeometryUtils.getObjRect(h, r)
                                    } if (t) {
                                        for (var a = (r.right - r.left) / 2 + r.left, i = (r.bottom - r.top) / 2 + r.top, c = 0; c < e.length; c++)
                                            if (this.selectLines[c].sel) {
                                                var o = e[c];
                                                this.selectLines[c].mx0 = o.x0 - a, this.selectLines[c].my0 = o.y0 - i
                                            } var s = {
                                                left: r.left,
                                                top: r.top,
                                                width: r.right - r.left,
                                                height: r.bottom - r.top,
                                                mx: 0,
                                                my: 0,
                                                z: 1
                                            };
                                        return s
                                    }
                                return null
                            },
                            operateAciton: function (e) {
                                this.selectLines.length > 0 ? this.deleteObj() : this.clearDraw(null)
                            },
                            deleteObj: function () {
                                this.addToHis();
                                for (var t = this.drawPoints.length - 1; t >= 0; t--) {
                                    var r = this.selectLines.length > t && this.selectLines[t].sel & this.selectMode;
                                    r && null != this.selectRect && (this.drawPoints.splice(t, 1), e("log", "drawPoints", this.drawPoints, " at sub/pages/draw/draw.js:950"))
                                }
                                this.resetSelectMode() || this.reDraw(this.drawPoints)
                            },
                            clearDraw: function (e) {
                                handDrawFileManager.clearDrawPointsHis(), this.points = [], this.drawPoints = [], this.drawPointCount = 0, this.linePtsSendSn = 0;
                                var t = uni.createCanvasContext("imgCanvas", this);
                                t.draw();
                                var r = uni.createCanvasContext("drawCanvas", this);
                                r.draw(), this.resetSelectMode() || this.setCanvasSub()
                            },
                            backDraw: function (t) {
                                if (handDrawFileManager.getDrawPointsHisCount() <= 0) return this.resetSelectMode(), void this.clearDraw();
                                var r = handDrawFileManager.popDrawPointsHis();
                                e("log", "his", r, " at sub/pages/draw/draw.js:981"), this.drawPoints = r.data,
                                    this.drawPointCount = handDrawGeometryUtils.getdrawPointsCnt(this.drawPoints),
                                    this.resetSelectMode() || this.reDraw(this.drawPoints)
                            },
                            reDraw: function (points) {
                                var canvasContext = uni.createCanvasContext("drawCanvas", this);
                                canvasContext.setLineWidth(1);
                                var drawConfig = {
                                    ctx: canvasContext,
                                    w: this.drawCanvas.w,
                                    h: this.drawCanvas.h,
                                    draw_line_type: j,
                                    colorSeg: this.colorSeg
                                },
                                    selectionState = {
                                        selectRect: this.selectRect,
                                        selectLines: this.selectLines,
                                        selectMode: this.selectMode
                                    },
                                    drawResults = handDrawGeometryUtils.drawPs(points, drawConfig, selectionState);
                                return canvasContext.draw(!0), this.selectMode || this.setCanvasSub(), drawResults
                            },
                            btnColorChange: function (e) {
                                var t = parseInt(e.currentTarget.dataset.tag);
                                this.lineColor = t;
                                for (var r = 0; r < this.selectLines.length; r++) this.selectLines[r].color = this.lineColor;
                                this.selectLines.length > 0 && (this.selectRect.ang = this.selectRect.lastAng - this.selectRect.startAng + this.selectRect.ang,
                                    this.selectRect.startAng = this.selectRect.lastAng, this.reDraw(this.drawPoints))
                            },
                            touchStart: function (e) {
                                this.points = null, this.points = [], this.lastLinePts = [];
                                var t = e.touches[0];
                                this.lastPoint.x = t.x, this.lastPoint.y = t.y, this.lastPoint.time = 0;
                                var r = uni.createCanvasContext("drawCanvasSub", this);
                                this.lineCtx = r, r.setLineDash([]);
                                var n = this.lineColor >= 8 ? 1 : this.lineColor;
                                r.setStrokeStyle(colors[n]), r.setFillStyle(colors[n]), this.points.push([this.lastPoint.x, this.lastPoint.y]);
                                var h = {
                                    x: this.lastPoint.x,
                                    y: this.lastPoint.y
                                };
                                this.lastLinePts.push({
                                    pt: h,
                                    color: 0,
                                    z: 1,
                                    time: (new Date).getTime()
                                }), r.moveTo(this.lastPoint.x, this.lastPoint.y)
                            },
                            touchMove: function (e) {
                                var t = e.touches[0],
                                    r = {
                                        x: t.x,
                                        y: t.y
                                    };
                                if (0 == this.lastPoint.time) return this.lastPoint.x = t.x, this.lastPoint.y = t.y, this.lastPoint.time = (new Date).getTime(), this.points = [], this.points.push([this.lastPoint.x, this.lastPoint.y]), void this.lineCtx.moveTo(this.lastPoint.x, this.lastPoint.y);
                                var n = (new Date).getTime();
                                if (!(n - this.lastPoint.time <= 2) && !(Math.abs(r.x - this.lastPoint.x) + Math.abs(r.y - this.lastPoint.y) < 3)) {
                                    this.lastPoint = {
                                        x: r.x,
                                        y: r.y,
                                        time: (new Date).getTime()
                                    }, this.sendLinePts(this.lastLinePts) && (this.lastLinePts = []);
                                    var h = this.lineColor >= 8 ? 1 : this.lineColor;
                                    this.lastLinePts.push({
                                        pt: r,
                                        color: h,
                                        z: 0,
                                        time: (new Date).getTime()
                                    }), this.points.push([this.lastPoint.x, this.lastPoint.y]), this.lineCtx.lineTo(this.lastPoint.x, this.lastPoint.y), this.lineCtx.stroke(), this.lineCtx.draw(!0), this.lineCtx.moveTo(this.lastPoint.x, this.lastPoint.y)
                                }
                            },
                            sendLinePts: function (t) {
                                var r = arguments.length > 1 && void 0 !== arguments[1] && arguments[1];
                                if (1 != this.sendLineMode) return !0;
                                if (t.length > 0 && (r || (new Date).getTime() - t[0].time > 50)) {
                                    if (r) {
                                        if (1 == t.length) {
                                            var n = this.lineColor >= 8 ? 1 : this.lineColor;
                                            t.push({
                                                pt: t[0].pt,
                                                color: n,
                                                z: 1
                                            })
                                        }
                                        t[t.length - 1].z = 1, e("log", "sendLinePts", t, " at sub/pages/draw/draw.js:1072")
                                    }
                                    var h = deviceCommandUtils.getDrawLineStr(t, this.linePtsSendSn),
                                        a = this.sendLineCmd(h, r);
                                    return e("log", "sendLinePts", t.length, a, this.linePtsSendSn, " at sub/pages/draw/draw.js:1076"), a
                                }
                                return !1
                            },
                            sendLineCmd: function (e, t) {
                                var r = this,
                                    n = bleDeviceControlUtils.gosend(!1, e);
                                return n && (this.linePtsSendSn = this.linePtsSendSn + 1), !t || n ? n : (setTimeout((function () {
                                    r.sendLineCmd(e, t)
                                }), 10), !1)
                            },
                            touchEnd: function (t) {
                                this.addToHis(), this.sendLinePts(this.lastLinePts, !0);
                                var r = uni.createCanvasContext("drawCanvas", this),
                                    n = handDrawGeometryUtils.covertPoints(this.points, this.lineColor, this.drawCanvas);
                                n["ang"] = 0, e("log", "res", n, " at sub/pages/draw/draw.js:1097");
                                var h = {
                                    ctx: r,
                                    w: this.drawCanvas.w,
                                    h: this.drawCanvas.h,
                                    draw_line_type: j,
                                    colorSeg: this.colorSeg
                                };
                                handDrawGeometryUtils.drawLine(h, n), this.drawPoints.push(n), r.draw(!0)
                            },
                            sendComplete: function (e, t) {
                                if (0 == e) {
                                    this.showSending = !0;
                                    var r = uni.createCanvasContext("progressCanvas", this);
                                    bleDeviceControlUtils.drawProgress(r, 300 * this.scUnit, t)
                                } else this.showSending = !1, this.lastCompleteTime = (new Date).getTime()
                            },
                            parmClose: function (e) {
                                this.$refs.popup.close(), this.showCanvas = !0, this.lastCmdTime = (new Date).getTime(), this.sendLastCmd("88")
                            },
                            chClick: function (e) {
                                this.cnfIdx = e, this.refreshChDraw()
                            },
                            drawChCanvas: function (e, t, r, n) {
                                var h = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : null,
                                    a = uni.createCanvasContext("chCanvas", this),
                                    i = e / 3;
                                a.setFontSize(i);
                                var c = e / 2,
                                    o = .95 * (this.chCanvas.w - e),
                                    s = (this.chCanvas.h - t) / 2 + c,
                                    l = o + e,
                                    p = s + t - e,
                                    d = o + c,
                                    g = s,
                                    j = l - c,
                                    x = p,
                                    V = 2 * this.scUnit,
                                    f = a.createLinearGradient(j, x + c, d, g - c);
                                f.addColorStop(0, "#112233"), f.addColorStop(1, "#1E374C"), a.setFillStyle(f), a.beginPath(), a.moveTo(l, p), a.arc(j, x, c, 0, 1 * Math.PI);
                                var F = t - 2 * c;
                                a.rect(l - e, p - F, e, F), a.moveTo(o, s), a.arc(d, g, c, Math.PI, 2 * Math.PI), a.fill();
                                var k = a.createLinearGradient(j, x + c, d, g - c);
                                k.addColorStop(0, "#008BD1"), k.addColorStop(1, "white"), a.setFillStyle(k), a.beginPath(), a.moveTo(o, s), a.arc(d, g, c, Math.PI, 2 * Math.PI), a.moveTo(l, p), a.arc(j, x, c, 0, 1 * Math.PI), a.beginPath();
                                var m = t / r,
                                    P = m * n;
                                if (P < c) {
                                    var u = c - P,
                                        X = c - Math.sqrt(Math.pow(c, 2) - Math.pow(u, 2)),
                                        N = handDrawGeometryUtils.lineTheta([l, p], [j, x], [l - X, p + u]);
                                    a.moveTo(l - X, p + u), a.arc(j, x, c, N, Math.PI - N), a.fill()
                                } else if (P <= t - c) {
                                    a.moveTo(l, p), a.arc(j, x, c, 0, 1 * Math.PI);
                                    var H = P - c;
                                    a.rect(l - e, p - H, e, H), a.fill()
                                } else {
                                    a.moveTo(l, p), a.arc(j, x, c, 0, 1 * Math.PI);
                                    var z = t - 2 * c;
                                    if (a.rect(l - e, p - z, e, z), n == r) a.moveTo(o, s), a.arc(d, g, c, Math.PI, 2 * Math.PI);
                                    else {
                                        var Q = P - (t - c),
                                            R = c - Math.sqrt(Math.pow(c, 2) - Math.pow(Q, 2)),
                                            v = handDrawGeometryUtils.lineTheta([o, s], [d, g], [o + R, s - Q]);
                                        a.moveTo(o + R, s - Q), a.arc(d, g, c, 2 * Math.PI - v, Math.PI + v)
                                    }
                                    a.fill()
                                }
                                if (a.beginPath(), a.setFontSize(26 * V), a.setFillStyle("white"), a.setShadow(5 * V, 5 * V, 5 * V, "rgba(0, 0, 0, 0.5)"), a.fillText(n + "", j - a.measureText(n + "").width / 2, g - c + t / 2 + i / 2), a.beginPath(), a.setFontSize(40 * V), a.fillText("+", d - a.measureText("+").width / 2, g + i / 2), a.fillText("-", j - a.measureText("-").width / 2, x + i), null != h) {
                                    var I = o,
                                        w = x + c;
                                    h(a, I, w, e, t, c, r, n)
                                }
                                a.draw()
                            },
                            addCnfValusAndSend: function (e) {
                                var t = this.pisObj.cnfValus[this.cnfIdx] + Math.floor(e);
                                t = t < 0 ? 0 : t, t = t > this.chDraw.max
                                    ? this.chDraw.max
                                    : t, this.pisObj.cnfValus[this.cnfIdx] != t
                                    && (this.$set(this.pisObj.cnfValus, this.cnfIdx, t),
                                        this.refreshChDraw(), this.lastCmdTime = (new Date).getTime(),
                                        this.sendLastCmd("66"))
                            },
                            callBackCh: function (e, t, r, n, h, a, i, c) {
                                if (this.cnfIdx in this.pisObjNote) {
                                    var o = 2 * this.scUnit,
                                        s = 10 * o;
                                    e.beginPath(), e.setLineWidth(1), e.setShadow(0, 0, 0, "rgba(0, 0, 0, 0)"), e.setStrokeStyle("#414339"), e.setFillStyle("#928F9F"), e.setFontSize(s);
                                    var l = this.pisObjNote[this.cnfIdx],
                                        p = .8 * t;
                                    t -= 1, e.moveTo(p, r), e.lineTo(t + a - 2, r), e.stroke();
                                    for (var d = 0, b = 0, g = !1, j = 0; j < l.length; j++) {
                                        l[j][0];
                                        var x = l[j][0] * this.chPer,
                                            V = l[j][1],
                                            f = Math.round(r - x - d);
                                        f < r - h && (f = r - h);
                                        var F = 0;
                                        x + d < a ? F = Math.round(a - Math.sqrt(Math.pow(a, 2) - Math.pow(a - x - d, 2))) : x + d > h - a && x + d < h ? F = Math.round(a - Math.sqrt(Math.pow(a, 2) - Math.pow(a - h + x + d, 2))) : x + d >= h && (F = a - 2), e.moveTo(p, f), e.lineTo(t + F, f), e.stroke(), b <= c && c < b + l[j][0] && (e.beginPath(), e.setFillStyle("#FFFFFF"), g = !0), b += l[j][0], e.fillText(V, p - e.measureText(V).width, r - d - (x - s / 2) / 2), d = x + d, g && (e.beginPath(), e.setFillStyle("#928F9F"))
                                    }
                                    e.stroke()
                                }
                            },
                            refreshChDraw: function () {
                                var e = this.pisObj.cnfValus[this.cnfIdx];
                                this.drawChCanvas(this.chDraw.w, this.chDraw.h, this.chDraw.max, e, this.callBackCh)
                            },
                            chTouchstart: function (e) {
                                var t = e.touches[0];
                                this.chBeginPoint = {
                                    x: t.x,
                                    y: t.y
                                }, this.chEndPoint = null, this.lastRefresh = 0
                            },
                            chTouchmove: function (e) {
                                var t = e.touches[0];
                                this.chEndPoint = {
                                    x: t.x,
                                    y: t.y
                                };
                                var r = (new Date).getTime();
                                if (r - this.lastRefresh > 100) {
                                    var n = Math.floor((this.chBeginPoint.y - this.chEndPoint.y) / this.chPer);
                                    Math.abs(n) >= 1 && (this.chBeginPoint = {
                                        x: this.chEndPoint.x,
                                        y: this.chEndPoint.y
                                    }, this.addCnfValusAndSend(n)), this.lastRefresh = r
                                }
                            },
                            chTouchend: function (e) {
                                if (null == this.chEndPoint) {
                                    var t = this.chBeginPoint.y > this.chCanvas.h / 2 ? -1 : 1;
                                    this.addCnfValusAndSend(t)
                                }
                                this.chEndPoint = null
                            },
                            slPointTimeChange: function (e) {
                                var t = e.detail.value;
                                this.pisObj.txPointTime = t, this.lastCmdTime = (new Date).getTime(), this.sendLastCmd("88")
                            },
                            parmReset: function (e) {
                                this.$set(this.pisObj, "cnfValus", [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), this.refreshChDraw(), this.lastCmdTime = (new Date).getTime(), this.sendLastCmd("66")
                            },
                            drawDone: function (t) {
                                if (!(this.drawPointCount <= 0)) {
                                    var r = (new Date).getTime();
                                    this.resetSelectMode(), e("log", "Send click time", r - this.lastCompleteTime, " at sub/pages/draw/draw.js:1355"),
                                        r - this.lastCompleteTime > 300
                                            ? this.sendCmd()
                                            : e("log", "Send click too frequently", " at sub/pages/draw/draw.js:1357")
                                }
                            },
                            parmSet: function (e) {
                                this.showCanvas = !1, this.$refs.popup.open("bottom");
                                var t = this,
                                    r = uni.createSelectorQuery().in(t);
                                this.$nextTick((function () {
                                    r.select("#chCanvas").boundingClientRect((function (e) {
                                        t.chCanvas.w = e.width, t.chCanvas.h = e.height;
                                        var r = .9 * t.chCanvas.h;
                                        t.chPer = r / 255, t.chDraw.w = t.chCanvas.w / 3, t.chDraw.h = r, t.refreshChDraw()
                                    })).exec()
                                }))
                            },
                            drawAddClick: function (t) {
                                var r = this,
                                    n = [this.$t("\u53e6\u5b58\u6587\u4ef6"), this.$t("\u9009\u62e9\u6587\u4ef6")];
                                this.currSelectedFile && n.push(this.$t("\u4fdd\u5b58\u6587\u4ef6")), uni.showActionSheet({
                                    itemList: n,
                                    success: function (e) {
                                        0 == e.tapIndex && r.drawAddAddClick(), 1 == e.tapIndex && r.drawAddSelectClick(), 2 == e.tapIndex && r.drawAddSaveClick()
                                    },
                                    fail: function (t) {
                                        e("log", t.errMsg, " at sub/pages/draw/draw.js:1392")
                                    }
                                })
                            },
                            saveDrawPic: function (fileName) {
                                var r = arguments.length > 1 && void 0 !== arguments[1] && arguments[1],
                                    n = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : -1;
                                e("log", "saveDrawPic", fileName, JSON.stringify(this.pisObj), " at sub/pages/draw/draw.js:1398");
                                var h = this,
                                    i = handDrawGeometryUtils.reSizeDrawPoints(h.drawPoints, this.drawCanvas.w, this.drawCanvas.h),
                                    c = JSON.parse(JSON.stringify(h.pisObj));
                                n >= 0 && (c.cnfValus[12] = n), handDrawFileManager.saveHandDrawImg(fileName, "", i, h.drawPointCount, c, h.features, r),
                                    r || (app.globalData.showModalTips(h.$t("\u4fdd\u5b58\u6210\u529f")), h.currSelectedFile = fileName)
                            },
                            saveDeskTop: function () {
                                this.saveDrawPic("saveDeskTopFile_001", !0)
                            },
                            restoreDeskTop: function () {
                                var e = this.getDrawByName("saveDeskTopFile_001", !0);
                                e && (this.needReDraw = !1, this.reDraw(this.drawPoints))
                            },
                            picNameInputCancelClick: function (e) {
                                this.showCanvas = !0, this.$refs.classNamePopup.close()
                            },
                            picNameNewInput: function (e) {
                                this.drawAddFileName = e.detail.value
                            },
                            picNameInputOkClick: function (e) {
                                var t = handDrawFileManager.combiFileName(this.handDrawClass[this.handDrawClassIdx], this.drawAddFileName);
                                if ("" == t) uni.showModal({
                                    content: this.$t("\u6587\u4ef6\u540d\u4e0d\u80fd\u4e3a\u7a7a"),
                                    showCancel: !1,
                                    success: function (e) { },
                                    fail: function (e) { },
                                    complete: function () { }
                                });
                                else {
                                    var r = handDrawFileManager.getHandDrawImg(t);
                                    r ? uni.showModal({
                                        content: this.$t("\u6587\u4ef6\u5df2\u5b58\u5728\uff0c\u8bf7\u91cd\u65b0\u8f93\u5165"),
                                        showCancel: !1,
                                        success: function (e) { },
                                        fail: function (e) { }
                                    }) : (this.saveDrawPic(t, !1, 3), this.picNameInputCancelClick(null))
                                }
                            },
                            checkAndAddImagFile: function (t) {
                                var r = this;
                                uni.showModal({
                                    title: this.$t("\u8bf7\u8f93\u5165\u6587\u4ef6\u540d"),
                                    placeholderText: t,
                                    editable: !0,
                                    success: function (n) {
                                        if (n.confirm) {
                                            var h = "" == n.content ? t : n.content;
                                            if ("" == h) uni.showModal({
                                                content: r.$t("\u6587\u4ef6\u540d\u4e0d\u80fd\u4e3a\u7a7a"),
                                                showCancel: !1,
                                                success: function (e) { },
                                                fail: function (e) { },
                                                complete: function () {
                                                    r.checkAndAddImagFile(t)
                                                }
                                            });
                                            else {
                                                var a = handDrawFileManager.getHandDrawImg(h);
                                                a ? uni.showModal({
                                                    content: r.$t("\u6587\u4ef6\u5df2\u5b58\u5728\uff0c\u662f\u5426\u7ee7\u7eed"),
                                                    showCancel: !0,
                                                    success: function (e) {
                                                        e.confirm ? r.saveDrawPic(h) : r.checkAndAddImagFile(t)
                                                    },
                                                    fail: function (e) { }
                                                }) : r.saveDrawPic(h)
                                            }
                                        } else n.cancel && e("log", "\u7528\u6237\u70b9\u51fb\u53d6\u6d88", " at sub/pages/draw/draw.js:1501")
                                    }
                                })
                            },
                            drawAddAddClick: function () {
                                if (0 != this.drawPointCount) {
                                    var e = handDrawFileManager.getHandDrawNames();
                                    e.count >= 50 ? app.globalData.showModalTips(this.$t("\u5df2\u8d85\u8fc7\u6700\u5927\u6587\u4ef6\u6570\u91cf ") + 50, !0) : e.noSpace ? app.globalData.showModalTips(this.$t("\u5b58\u50a8\u7a7a\u95f4\u4e0d\u8db3"), !0) : (this.handDrawClassName = handDrawFileManager.getHandDrawClassName(), this.$set(this, "handDrawClassIdx", 0), this.drawAddFileName = handDrawFileManager.getNewFileName(), this.showCanvas = !1, this.$refs.classNamePopup.open("center"))
                                } else app.globalData.showModalTips(this.$t("\u8bf7\u5148\u7ed8\u5236\u56fe\u6848"))
                            },
                            handDrawClassPickerChange: function (e) {
                                this.handDrawClassIdx = parseInt(e.detail.value)
                            },
                            getDrawByName: function (e) {
                                var t = arguments.length > 1 && void 0 !== arguments[1] && arguments[1],
                                    r = handDrawFileManager.getHandDrawImg(e, t);
                                if (r) {
                                    if (r.pointCnt <= 0) return !1;
                                    if (r.pointCnt > 800) return app.globalData.showModalTips(this.$t("Exceeds the maximum number of points") + 800, !0), !1;
                                    this.drawPointCount = r.pointCnt, this.addToHis();
                                    var n = handDrawGeometryUtils.reSizeDrawPoints(r.drawPoints, handDrawGeometryUtils.defaultWith, handDrawGeometryUtils.defaultHeight, this.drawCanvas.w, this.drawCanvas.h);
                                    return this.drawPoints = n,
                                        r.pisObj && this.$set(this, "pisObj", r.pisObj), this.selectLines = [], this.selectMode = !1, this.needReDraw = !0, !0
                                }
                                return !1
                            },
                            drawAddSaveClick: function () {
                                if (0 != this.drawPointCount) {
                                    var e = this;
                                    uni.showModal({
                                        content: e.$t("\u4fdd\u5b58\u6587\u4ef6") + this.currSelectedFile + "?",
                                        showCancel: !0,
                                        success: function (t) {
                                            t.confirm && e.saveDrawPic(e.currSelectedFile)
                                        },
                                        fail: function (e) { }
                                    })
                                } else app.globalData.showModalTips(this.$t("\u8bf7\u5148\u7ed8\u5236\u56fe\u6848"))
                            },
                            drawAddSelectClick: function () {
                                var t = this;
                                uni.navigateTo({
                                    url: "/sub/pages/files/files",
                                    events: {
                                        acceptDataFromOpenedPage: function (r) {
                                            e("log", "acceptDataFromOpenedPage", r, " at sub/pages/draw/draw.js:1580"), t.currSelectedFile = r.fileName, t.getDrawByName(r.fileName)
                                        }
                                    },
                                    success: function (e) {
                                        e.eventChannel.emit("acceptDataFromOpenerPage", {
                                            callFrom: "draw"
                                        })
                                    },
                                    fail: function (t) {
                                        e("log", t, " at sub/pages/draw/draw.js:1589")
                                    }
                                })
                            },
                            tipsCheckboxChange: function (e) {
                                var t = e.detail.value;
                                this.showTips = !t.includes("tips"), app.globalData.saveTipsParm(this.showTips)
                            },
                            tipOpen: function () {
                                this.showTips && (this.showCanvas = !1, this.$refs.tips.open("center"))
                            },
                            tipsClose: function (e) {
                                this.$refs.tips.close(), this.showCanvas = !0
                            },
                            onBtnSetTouchStart: function (e) {
                                this.startPosition.x = e.touches[0].clientX - this.position.x, this.startPosition.y = e.touches[0].clientY - this.position.y
                            },
                            onBtnSetTouchMove: function (e) {
                                this.position.x = e.touches[0].clientX - this.startPosition.x, this.position.y = e.touches[0].clientY - this.startPosition.y
                            },
                            onBtnSetTouchEnd: function () { },
                            onBtnSetClick: function (e) {
                                uni.navigateTo({
                                    url: "/pages/subset/subset"
                                })
                            },
                            chooseImag: function (t) {
                                var r = this;
                                this.resetSelectMode(), app.globalData.img_selecting = !0, uni.chooseImage({
                                    count: 1,
                                    sizeType: ["original", "compressed"],
                                    sourceType: ["album", "camera"],
                                    success: function (e) {
                                        var t = e.tempFilePaths[0];
                                        uni.navigateTo({
                                            url: "/sub/pages/cover/cover",
                                            events: {
                                                acceptDataFromOpenedPage: function (e) {
                                                    if (null != e) {
                                                        if (0 == e.mode && null != e.data) {
                                                            var t = uni.createCanvasContext("imgCanvas", r);
                                                            t.drawImage(e.data, 0, 0, r.drawCanvas.w, r.drawCanvas.h), t.draw()
                                                        }
                                                        if (1 == e.mode && null != e.data) {
                                                            r.obj = textLineVectorizer.dealImgLines(e.data), r.drawMode = 8888;
                                                            var n = r.drawCanvas.w / e.size * .8;
                                                            r.objParm = {
                                                                x0: r.drawCanvas.w / 2,
                                                                y0: r.drawCanvas.w / 2,
                                                                z: n,
                                                                ang: 0,
                                                                ps: r.obj,
                                                                lineColor: r.lineColor
                                                            }, r.touchEndSub()
                                                        }
                                                    }
                                                }
                                            },
                                            success: function (e) {
                                                e.eventChannel.emit("acceptDataFromOpenerPage", t)
                                            }
                                        })
                                    },
                                    complete: function (t) {
                                        e("log", "app.globalData.img_selecting", app.globalData.img_selecting, " at sub/pages/draw/draw.js:1668"), app.globalData.img_selecting = !1
                                    }
                                })
                            }
                        }
                    };
                t.default = x
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "scenePatternEditorPageComponent ": function (e, t, r) {
            "use strict";
            (function (e) {
                var n = r("esModuleInteropHelper");
                Object.defineProperty(t, "__esModule", {
                    value: !0
                }), t.default = void 0;
                var h = n(r("uniPopupComponentExportWrapper")),
                    a = getApp(),
                    i = r("deviceCommandUtils "),
                    c = r("bleDeviceControlUtils "),
                    o = ["black", "red", "green", "blue", "yellow", "#00FFFF", "purple", "white"],
                    s = {
                        data: function () {
                            var e = a.globalData.getDeviceFeatures(),
                                t = e.xyCnf ? 18 : 12;
                            return {
                                screen_width: a.globalData.screen_width_str,
                                scUnit: a.globalData.screen_width_float,
                                rtl: a.globalData.rtl,
                                ntitle: this.$t("\u573a\u666f\u7f16\u8f91"),
                                loadObjTitel: this.$t("\u52a0\u8f7d\u4e2d") + "...",
                                imageDrawOffset: 0,
                                imageShowCount: 0,
                                features: e,
                                popupShow: !1,
                                imageListViewHeight: 0,
                                imgArrays: [],
                                lastRefresh: 0,
                                lastCmdTime: 0,
                                lastSendtime: 0,
                                chPer: 1,
                                chBeginPoint: {
                                    x: 0,
                                    y: 0
                                },
                                chEndPoint: {
                                    x: 0,
                                    y: 0
                                },
                                pisObjNote: {
                                    0: [
                                        [256, this.$t("\u56fe\u6848\u9009\u62e9")]
                                    ],
                                    1: [
                                        [25, this.$t("\u76f4\u7ebf\u7c7b\u56fe\u6848")],
                                        [25, this.$t("\u5706\u5f27\u7c7b\u56fe\u6848")],
                                        [25, this.$t("\u4eae\u70b9\u56fe\u6848")],
                                        [25, this.$t("\u6253\u70b9\u56fe\u6848")],
                                        [25, this.$t("\u5723\u8bde\u56fe\u6848")],
                                        [25, this.$t("\u52a8\u753b\u7ec4\u522b1")],
                                        [25, this.$t("\u52a8\u753b\u7ec4\u522b2")],
                                        [25, this.$t("\u52a8\u753b\u7ec4\u522b3")],
                                        [25, this.$t("\u52a8\u753b\u7ec4\u522b4")],
                                        [31, this.$t("\u52a8\u753b\u7ec4\u522b5")]
                                    ],
                                    2: [
                                        [10, this.$t("\u767d\u8272")],
                                        [10, this.$t("\u7ea2\u8272")],
                                        [10, this.$t("\u84dd\u8272")],
                                        [10, this.$t("\u7c89\u8272")],
                                        [10, this.$t("\u9752\u8272")],
                                        [10, this.$t("\u9ec4\u8272")],
                                        [10, this.$t("\u7eff\u8272")],
                                        [10, this.$t("\u6574\u4f53\u989c\u8272\u6362\u8272")],
                                        [13, this.$t("\u4e03\u5f69\u8679\u989c\u8272")],
                                        [18, this.$t("2\u5206\u6bb5\u989c\u8272")],
                                        [21, this.$t("3\u5206\u6bb5\u989c\u8272")],
                                        [18, this.$t("4\u5206\u6bb5\u989c\u8272")],
                                        [33, this.$t("8\u5206\u6bb5\u989c\u8272")],
                                        [36, this.$t("16\u5206\u6bb5\u989c\u8272")],
                                        [35, this.$t("32\u5206\u6bb5\u989c\u8272")],
                                        [2, this.$t("\u989c\u8272\u6e10\u7ed8")]
                                    ],
                                    3: [
                                        [10, this.$t("\u4e0d\u6d41\u6c34")],
                                        [118, this.$t("\u6b63\u5411\u6d41\u6c34")],
                                        [128, this.$t("\u53cd\u5411\u6d41\u6c34")]
                                    ],
                                    4: [
                                        [256, this.$t("\u56fe\u6848\u5927\u5c0f")]
                                    ],
                                    5: [
                                        [16, this.$t("\u7f29\u653eManual\u9009\u62e9")],
                                        [40, this.$t("\u7531\u5c0f\u5230\u5927\u7f29\u653e")],
                                        [40, this.$t("\u7531\u5927\u5230\u5c0f\u7f29\u653e")],
                                        [40, this.$t("\u5927\u5c0f\u5faa\u73af\u7f29\u653e")],
                                        [40, this.$t("\u4e0d\u89c4\u5219\u7f29\u653e\u4e00")],
                                        [40, this.$t("\u4e0d\u89c4\u5219\u7f29\u653e\u4e8c")],
                                        [40, this.$t("\u4e0d\u89c4\u5219\u7f29\u653e\u4e09")]
                                    ],
                                    6: [
                                        [128, this.$t("\u65cb\u8f6c\u89d2\u5ea6")],
                                        [64, this.$t("\u6b63\u65cb\u8f6c\u901f\u5ea6")],
                                        [64, this.$t("\u53cd\u65cb\u8f6c\u901f\u5ea6")]
                                    ],
                                    7: [
                                        [128, this.$t("\u6c34\u5e73\u7ffb\u8f6c\u4f4d\u7f6e")],
                                        [128, this.$t("\u6c34\u5e73\u7ffb\u8f6c\u901f\u5ea6")]
                                    ],
                                    8: [
                                        [128, this.$t("\u5782\u76f4\u7ffb\u8f6c\u4f4d\u7f6e")],
                                        [128, this.$t("\u5782\u76f4\u7ffb\u8f6c\u901f\u5ea6")]
                                    ],
                                    9: [
                                        [128, this.$t("\u6c34\u5e73\u4f4d\u7f6e\u65cb\u8f6c")],
                                        [128, this.$t("\u6c34\u5e73\u79fb\u52a8")]
                                    ],
                                    10: [
                                        [128, this.$t("\u5782\u76f4\u4f4d\u7f6e\u65cb\u8f6c")],
                                        [128, this.$t("\u5782\u76f4\u79fb\u52a8")]
                                    ],
                                    11: [
                                        [2, this.$t("\u65e0\u6ce2\u6d6a")],
                                        [31, this.$t("\u6ce2\u6d6a\u5e45\u5ea61")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea62")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea63")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea64")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea65")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea66")],
                                        [32, this.$t("\u6ce2\u6d6a\u5e45\u5ea67")],
                                        [31, this.$t("\u6ce2\u6d6a\u5e45\u5ea68")]
                                    ],
                                    12: [
                                        [2, this.$t("\u65e0\u6e10\u7ed8")],
                                        [62, this.$t("Manual\u6e10\u7ed81")],
                                        [64, this.$t("Manual\u6e10\u7ed82")],
                                        [26, this.$t("Automatic\u6e10\u7ed81")],
                                        [26, this.$t("Automatic\u6e10\u7ed82")],
                                        [26, this.$t("Automatic\u6e10\u7ed83")],
                                        [50, this.$t("Automatic\u6e10\u7ed84")]
                                    ],
                                    13: [
                                        [256, this.$t("\u6c34\u5e73\u7535\u673a")]
                                    ],
                                    14: [
                                        [256, this.$t("\u6c34\u5e73\u5fae\u8c03")]
                                    ],
                                    15: [
                                        [256, this.$t("\u5782\u76f4\u7535\u673a")]
                                    ],
                                    16: [
                                        [256, this.$t("\u5782\u76f4\u5fae\u8c03")]
                                    ],
                                    17: [
                                        [256, this.$t("\u7535\u673a\u901f\u5ea6")]
                                    ]
                                },
                                showPisCanvas: !0,
                                fontWidth: 100,
                                pisIdx: 0,
                                cnfIdx: 1,
                                cnfGroup: 0,
                                defaultPis: null,
                                pisObj: {
                                    playTime: 10,
                                    cnfValus: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                                },
                                pisSelectedIdx: -1,
                                pisSelectedGroup: 0,
                                loading: 0,
                                pisObjArray: [],
                                myCanvasSize: {
                                    w: 0,
                                    h: 0
                                },
                                chDraw: {
                                    w: 0,
                                    h: 0,
                                    max: 255
                                },
                                chCanvas: {
                                    w: 0,
                                    h: 0
                                },
                                cnfList: [{
                                    name: this.$t("\u56fe\u5f62\u5206\u7ec4"),
                                    value: 255,
                                    idx: 1
                                }, {
                                    name: this.$t("\u56fe\u5f62"),
                                    value: 255,
                                    idx: 0
                                }, {
                                    name: this.$t("\u989c\u8272"),
                                    value: 255,
                                    idx: 2
                                }, {
                                    name: this.$t("\u989c\u8272\u6d41\u6c34"),
                                    value: 255,
                                    idx: 3
                                }, {
                                    name: this.$t("\u56fe\u5f62\u5927\u5c0f"),
                                    value: 255,
                                    idx: 4
                                }, {
                                    name: this.$t("\u56fe\u5f62\u7f29\u653e"),
                                    value: 255,
                                    idx: 5
                                }, {
                                    name: this.$t("\u56fe\u5f62\u65cb\u8f6c"),
                                    value: 255,
                                    idx: 6
                                }, {
                                    name: this.$t("\u6c34\u5e73\u7ffb\u8f6c"),
                                    value: 255,
                                    idx: 7
                                }, {
                                    name: this.$t("\u5782\u76f4\u7ffb\u8f6c"),
                                    value: 255,
                                    idx: 8
                                }, {
                                    name: this.$t("\u6c34\u5e73\u79fb\u52a8"),
                                    value: 255,
                                    idx: 9
                                }, {
                                    name: this.$t("\u5782\u76f4\u79fb\u52a8"),
                                    value: 255,
                                    idx: 10
                                }, {
                                    name: this.$t("\u6ce2\u6d6a"),
                                    value: 255,
                                    idx: 11
                                }, {
                                    name: this.$t("\u6e10\u7ed8"),
                                    value: 255,
                                    idx: 12
                                }, {
                                    name: this.$t("\u65f6\u95f4"),
                                    value: 255,
                                    idx: 13
                                }, {
                                    name: this.$t("\u6c34\u5e73\u7535\u673a"),
                                    value: 255,
                                    idx: 14
                                }, {
                                    name: this.$t("\u6c34\u5e73\u5fae\u8c03"),
                                    value: 255,
                                    idx: 15
                                }, {
                                    name: this.$t("\u5782\u76f4\u7535\u673a"),
                                    value: 255,
                                    idx: 16
                                }, {
                                    name: this.$t("\u5782\u76f4\u5fae\u8c03"),
                                    value: 255,
                                    idx: 17
                                }, {
                                    name: this.$t("\u7535\u673a\u901f\u5ea6"),
                                    value: 255,
                                    idx: 18
                                }],
                                showCnfEndIdx: t,
                                slCnfSended: !1,
                                slCnfChangingTimer: null
                            }
                        },
                        computed: {
                            playTime: {
                                get: function () {
                                    return this.pisObj.playTime
                                },
                                set: function (e) {
                                    var t = parseInt(e),
                                        r = this.pisObj.playTime;
                                    if (!(r > 25.5))
                                        if (this.$set(this.pisObj, "playTime", t), t > 25.5) {
                                            var n = this;
                                            setTimeout((function () {
                                                n.$set(n.pisObj, "playTime", r)
                                            }), 100)
                                        } else this.sendCmd()
                                }
                            }
                        },
                        components: {
                            uniPopup: h.default
                        },
                        onLoad: function (t) {
                            e("log", "onload", " at sub2/pages/pis/pis.js:133");
                            var r = this.getPicArray();
                            this.pisObjArray = r.getPicArrayInfo(), e("log", "pisObjArray", this.pisObjArray, " at sub2/pages/pis/pis.js:136"), this.initData();
                            var n = this,
                                h = this.getOpenerEventChannel();
                            h.on("acceptDataFromOpenerPage", (function (e) {
                                n.pisObj = e.pis, n.pisIdx = e.idx, n.imgArrays = e.imgArrays, n.defaultPis = e.defaultPis, n.$nextTick((function () {
                                    n.sendCmd(), setTimeout((function () {
                                        var e = uni.createSelectorQuery().in(n);
                                        e.select("#myCanvas").boundingClientRect((function (e) {
                                            n.myCanvasSize.w = e.width, n.myCanvasSize.h = e.height, n.myCanvasDraw()
                                        })).exec(), n.initChCanvas(), n.startImageCount()
                                    }), 100)
                                }))
                            }))
                        },
                        onUnload: function () {
                            var e = this.getOpenerEventChannel();
                            e.emit("acceptDataFromOpenedPage", {
                                pis: this.pisObj,
                                imgArrays: this.imgArrays
                            })
                        },
                        onReady: function () { },
                        onShow: function () { },
                        onHide: function () { },
                        methods: {
                            getPicArray: function () {
                                if (this.features.ilda) {
                                    var e = r("lineShapes");
                                    return e
                                }
                                var shapePatternTemplates = r("shapePatternTemplates");
                                return shapePatternTemplates
                            },
                            sendCmd: function () {
                                var e = (new Date).getTime(),
                                    command = i.getPisCmdStr(this.pisIdx, this.pisObj, {
                                        features: this.features
                                    }),
                                    r = c.gosend(!1, command);
                                return r && (this.lastSendtime = e), r
                            },
                            initData: function () {
                                this.features.ilda && (this.pisObjNote[1] = [
                                    [25, this.$t("\u76f4\u7ebf\u7c7b\u56fe\u6848")],
                                    [25, this.$t("\u5706\u5f27\u7c7b\u56fe\u6848")],
                                    [25, this.$t("\u4eae\u70b9\u56fe\u6848")],
                                    [25, this.$t("\u6253\u70b9\u56fe\u6848")],
                                    [25, this.$t("\u4fdd\u7559")],
                                    [25, this.$t("\u52a8\u753b\u7ec4\u522b1")],
                                    [25, this.$t("\u52a8\u753b\u7ec4\u522b2")],
                                    [25, this.$t("\u52a8\u753b\u7ec4\u522b3")],
                                    [25, this.$t("\u52a8\u753b\u7ec4\u522b4")],
                                    [31, this.$t("\u52a8\u753b\u7ec4\u522b5")]
                                ], this.pisObjNote[2] = [
                                    [5, this.$t("\u767d\u8272")],
                                    [5, this.$t("\u7ea2\u8272")],
                                    [5, this.$t("\u84dd\u8272")],
                                    [5, this.$t("\u7c89\u8272")],
                                    [5, this.$t("\u9752\u8272")],
                                    [5, this.$t("\u9ec4\u8272")],
                                    [5, this.$t("\u7eff\u8272")],
                                    [5, this.$t("\u6574\u4f53\u989c\u8272\u6362\u8272")],
                                    [5, this.$t("\u56fe\u6848\u521d\u59cb\u989c\u8272")],
                                    [2, this.$t("\u4e03\u5f69\u8679\u989c\u8272")],
                                    [20, this.$t("2\u5206\u6bb5\u989c\u8272")],
                                    [30, this.$t("3\u5206\u6bb5\u989c\u8272")],
                                    [30, this.$t("4\u5206\u6bb5\u989c\u8272")],
                                    [24, this.$t("8\u5206\u6bb5\u989c\u8272")],
                                    [24, this.$t("16\u5206\u6bb5\u989c\u8272")],
                                    [40, this.$t("32\u5206\u6bb5\u989c\u8272")],
                                    [33, this.$t("\u6df7\u8272")],
                                    [8, this.$t("\u989c\u8272\u6e10\u7ed8")]
                                ])
                            },
                            initChCanvas: function () {
                                var e = this,
                                    t = uni.createSelectorQuery().in(e);
                                t.select("#chCanvas").boundingClientRect((function (t) {
                                    if (e.chCanvas.w != t.width || e.chCanvas.h != t.height) {
                                        e.chCanvas.w = t.width, e.chCanvas.h = t.height;
                                        var r = .9 * e.chCanvas.h;
                                        e.chPer = r / 255, e.chDraw.w = e.chCanvas.w / 3, e.chDraw.h = r, e.refreshChDraw(), setTimeout((function () {
                                            e.initChCanvas()
                                        }), 100)
                                    }
                                })).exec()
                            },
                            startImageCount: function () {
                                var e = this;
                                setInterval((function () {
                                    e.popupShow && 4 == e.pisSelectedGroup && (e.imageShowCount = e.imageShowCount + 1, e.imageShowCount >= 1e3 && (e.imageShowCount = 0))
                                }), 400)
                            },
                            refreshChDraw: function () {
                                var e = this.pisObj.cnfValus[this.cnfIdx];
                                this.drawChCanvas(this.chDraw.w, this.chDraw.h, this.chDraw.max, e, this.callBackCh)
                            },
                            lineTheta: function (e, t, r) {
                                var n = {
                                    x: e[0] - t[0],
                                    y: e[1] - t[1]
                                },
                                    h = {
                                        x: r[0] - t[0],
                                        y: r[1] - t[1]
                                    },
                                    a = n.x * h.x + n.y * h.y,
                                    i = Math.sqrt(Math.pow(n.x, 2) + Math.pow(n.y, 2)),
                                    c = Math.sqrt(Math.pow(h.x, 2) + Math.pow(h.y, 2)),
                                    o = Math.acos(a / (i * c));
                                return o
                            },
                            myCanvasDraw: function () {
                                if (this.popupShow) this.myCanvasClear();
                                else {
                                    var e = this,
                                        t = uni.createCanvasContext("myCanvas", e),
                                        r = this.convertPic255ToIdx(this.pisObj.cnfValus[1], this.pisObj.cnfValus[0]),
                                        n = r.iidx,
                                        h = r.igroup;
                                    4 == h ? (this.imageDrawOffset++, this.imageDrawOffset = this.imageDrawOffset >= 1e3 ? 0 : this.imageDrawOffset, this.imageDrawOffset = 20 == n || 22 == n ? this.imageDrawOffset % 2 : 24 == n ? this.imageDrawOffset % 6 : 30 == n ? this.imageDrawOffset % 40 : -1) : this.imageDrawOffset = -1;
                                    var a = this.getPicArray(),
                                        i = a.picArray,
                                        c = e.myCanvasSize.w > e.myCanvasSize.h ? e.myCanvasSize.h : e.myCanvasSize.w,
                                        o = e.myCanvasSize.w / 2,
                                        s = e.myCanvasSize.h / 2;
                                    if (5 == h || 6 == h) {
                                        var l = this.$t(i[h].label) + n,
                                            p = c / 4;
                                        return this.drawABtag(t, l, o, s, p), void t.draw()
                                    }
                                    if (5 <= h || -1 == n || n >= i[h].arr.length) t.draw();
                                    else {
                                        var d = i[h].arr[n];
                                        this.imageDrawOffset >= 0 && (d = i[h].arr[n + this.imageDrawOffset]);
                                        var b = c / 800 * .8;
                                        this.drawObj(t, d, o, s, b), t.draw(), this.imageDrawOffset >= 0 && setTimeout((function () {
                                            e.myCanvasDraw()
                                        }), 400)
                                    }
                                }
                            },
                            drawABtag: function (e, t, r, n, h) {
                                e.beginPath(), e.setFillStyle("red"), e.setStrokeStyle("red"), e.setFontSize(h), e.strokeText(t, r - e.measureText(t).width / 2, n + h / 3), e.stroke(), e.fill()
                            },
                            sendLastCmd: function () {
                                var e = this;
                                e.lastCmdTime <= e.lastSendtime || setTimeout((function () {
                                    e.lastCmdTime <= e.lastSendtime || (e.sendCmd(), e.lastCmdTime > e.lastSendtime && e.sendLastCmd())
                                }), 10)
                            },
                            addCnfValusAndSend: function (e) {
                                var t = this.pisObj.cnfValus[this.cnfIdx] + Math.floor(e);
                                t = t < 0 ? 0 : t, t = t > this.chDraw.max ? this.chDraw.max : t, this.pisObj.cnfValus[this.cnfIdx] != t && (this.$set(this.pisObj.cnfValus, this.cnfIdx, t), this.refreshChDraw(), this.lastCmdTime = (new Date).getTime(), this.sendLastCmd(), 0 != this.cnfIdx && 1 != this.cnfIdx || this.myCanvasDraw())
                            },
                            chTouchstart: function (e) {
                                var t = e.touches[0];
                                this.chBeginPoint = {
                                    x: t.x,
                                    y: t.y
                                }, this.chEndPoint = null, this.lastRefresh = 0
                            },
                            chTouchmove: function (e) {
                                var t = e.touches[0];
                                this.chEndPoint = {
                                    x: t.x,
                                    y: t.y
                                };
                                var r = (new Date).getTime();
                                if (r - this.lastRefresh > 100) {
                                    var n = Math.floor((this.chBeginPoint.y - this.chEndPoint.y) / this.chPer);
                                    Math.abs(n) >= 1 && (this.chBeginPoint = {
                                        x: this.chEndPoint.x,
                                        y: this.chEndPoint.y
                                    }, this.addCnfValusAndSend(n)), this.lastRefresh = r
                                }
                            },
                            chTouchend: function (e) {
                                if (null == this.chEndPoint) {
                                    var t = this.chBeginPoint.y > this.chCanvas.h / 2 ? -1 : 1;
                                    this.addCnfValusAndSend(t)
                                }
                                this.chEndPoint = null
                            },
                            callBackCh: function (e, t, r, n, h, a, i, c) {
                                if (this.cnfIdx in this.pisObjNote) {
                                    var o = 2 * this.scUnit,
                                        s = 10 * o,
                                        l = .65 * s;
                                    e.beginPath(), e.setLineWidth(1), e.setShadow(0, 0, 0, "rgba(0, 0, 0, 0)"), e.setStrokeStyle("#414339"), e.setFillStyle("#928F9F"), e.setFontSize(s);
                                    var p = this.pisObjNote[this.cnfIdx],
                                        d = .8 * t;
                                    t -= 1, e.moveTo(d, r), e.lineTo(t + a - 2, r), e.stroke();
                                    for (var b = 0, g = 0, j = !1, x = 0; x < p.length; x++) {
                                        p[x][0];
                                        var V = p[x][0] * this.chPer,
                                            f = p[x][1],
                                            F = Math.round(r - V - b);
                                        F < r - h && (F = r - h);
                                        var k = 0;
                                        V + b < a ? k = Math.round(a - Math.sqrt(Math.pow(a, 2) - Math.pow(a - V - b, 2))) : V + b > h - a && V + b < h ? k = Math.round(a - Math.sqrt(Math.pow(a, 2) - Math.pow(a - h + V + b, 2))) : V + b >= h && (k = a - 2), e.moveTo(d, F), e.lineTo(t + k, F), e.stroke(), g <= c && c < g + p[x][0] && (e.beginPath(), e.setFillStyle("#FFFFFF"), j = !0), g += p[x][0];
                                        var m = s;
                                        V < s && (m = l), e.setFontSize(m), V < m ? e.fillText(f, d - e.measureText(f).width, r - b - V / 2) : e.fillText(f, d - e.measureText(f).width, r - b - (V - m / 2) / 2), b = V + b, j && (e.beginPath(), e.setFillStyle("#928F9F"))
                                    }
                                    e.stroke()
                                }
                            },
                            drawChCanvas: function (e, t, r, n) {
                                var h = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : null,
                                    a = uni.createCanvasContext("chCanvas", this),
                                    i = e / 3;
                                a.setFontSize(i);
                                var c = e / 2,
                                    o = .95 * (this.chCanvas.w - e),
                                    s = (this.chCanvas.h - t) / 2 + c,
                                    l = o + e,
                                    p = s + t - e,
                                    d = o + c,
                                    b = s,
                                    g = l - c,
                                    j = p,
                                    x = 2 * this.scUnit,
                                    V = a.createLinearGradient(g, j + c, d, b - c);
                                V.addColorStop(0, "#112233"), V.addColorStop(1, "#1E374C"), a.setFillStyle(V), a.beginPath(), a.moveTo(l, p), a.arc(g, j, c, 0, 1 * Math.PI);
                                var f = t - 2 * c;
                                a.rect(l - e, p - f, e, f), a.moveTo(o, s), a.arc(d, b, c, Math.PI, 2 * Math.PI), a.fill();
                                var F = a.createLinearGradient(g, j + c, d, b - c);
                                F.addColorStop(0, "#008BD1"), F.addColorStop(1, "white"), a.setFillStyle(F), a.beginPath(), a.moveTo(o, s), a.arc(d, b, c, Math.PI, 2 * Math.PI), a.moveTo(l, p), a.arc(g, j, c, 0, 1 * Math.PI), a.beginPath();
                                var k = t / r,
                                    m = k * n;
                                if (m < c) {
                                    var P = c - m,
                                        u = c - Math.sqrt(Math.pow(c, 2) - Math.pow(P, 2)),
                                        X = this.lineTheta([l, p], [g, j], [l - u, p + P]);
                                    a.moveTo(l - u, p + P), a.arc(g, j, c, X, Math.PI - X), a.fill()
                                } else if (m <= t - c) {
                                    a.moveTo(l, p), a.arc(g, j, c, 0, 1 * Math.PI);
                                    var N = m - c;
                                    a.rect(l - e, p - N, e, N), a.fill()
                                } else {
                                    a.moveTo(l, p), a.arc(g, j, c, 0, 1 * Math.PI);
                                    var H = t - 2 * c;
                                    if (a.rect(l - e, p - H, e, H), n == r) a.moveTo(o, s), a.arc(d, b, c, Math.PI, 2 * Math.PI);
                                    else {
                                        var z = m - (t - c),
                                            Q = c - Math.sqrt(Math.pow(c, 2) - Math.pow(z, 2)),
                                            R = this.lineTheta([o, s], [d, b], [o + Q, s - z]);
                                        a.moveTo(o + Q, s - z), a.arc(d, b, c, 2 * Math.PI - R, Math.PI + R)
                                    }
                                    a.fill()
                                }
                                if (a.beginPath(), a.setFontSize(26 * x), a.setFillStyle("white"), a.setShadow(5 * x, 5 * x, 5 * x, "rgba(0, 0, 0, 0.5)"), a.fillText(n + "", g - a.measureText(n + "").width / 2, b - c + t / 2 + i / 2), a.beginPath(), a.setFontSize(40 * x), a.fillText("+", d - a.measureText("+").width / 2, b + i / 2), a.fillText("-", g - a.measureText("-").width / 2, j + i), null != h) {
                                    var v = o,
                                        I = j + c;
                                    h(a, v, I, e, t, c, r, n)
                                }
                                a.draw()
                            },
                            chClick: function (e) {
                                this.cnfIdx = e, this.refreshChDraw()
                            },
                            drawObj: function (e, t, r, n, h) {
                                e.beginPath();
                                for (var a = 0; a < t.length; a++) {
                                    var i = [];
                                    Object.assign(i, t[a]), i[0] = i[0] * h + r, i[1] = -i[1] * h + n;
                                    var c = null;
                                    a < t.length - 1 && (c = t[a + 1]), null != c && i[2] != c[2] ? (e.setStrokeStyle(o[i[2]]), 0 != i[2] && e.lineTo(i[0], i[1]), e.stroke(), e.beginPath(), e.moveTo(i[0], i[1])) : (e.lineTo(i[0], i[1]), null == c && e.setStrokeStyle(o[i[2]]))
                                }
                                e.stroke();
                                var s = null;
                                e.beginPath();
                                for (var l = 0; l < t.length; l++) {
                                    var p = [];
                                    Object.assign(p, t[l]), p[0] = p[0] * h + r, p[1] = -p[1] * h + n;
                                    var d = null;
                                    l < t.length - 1 && (d = t[l + 1]), null != d && p[2] != d[2] ? null != s && s[0] == p[0] && s[1] == p[1] && (e.setStrokeStyle(o[p[2]]), e.setFillStyle(o[p[2]]), e.moveTo(p[0], p[1]), e.arc(p[0], p[1], 1, 0, 2 * Math.PI), e.stroke(), e.fill(), e.beginPath()) : null != s && s[0] == p[0] && s[1] == p[1] && (e.setStrokeStyle(o[p[2]]), e.setFillStyle(o[p[2]]), e.moveTo(p[0], p[1]), e.arc(p[0], p[1], 1, 0, 2 * Math.PI), e.stroke()), s = p
                                }
                                e.fill()
                            },
                            pisCanvasClear: function () {
                                var e = uni.createCanvasContext("pisCanvas", this);
                                e.draw()
                            },
                            myCanvasClear: function () {
                                var e = uni.createCanvasContext("myCanvas", this);
                                e.draw()
                            },
                            createImg: function (t, r, n) {
                                if (this.pisSelectedGroup == t) {
                                    var h = this;
                                    if (0 == h.imgArrays.length)
                                        for (var a = 0; a < n.length; a++) {
                                            var i = new Array(n[a].arr.length).fill("");
                                            h.imgArrays.push(i)
                                        }
                                    if (this.popupShow)
                                        if (e("log", "group, idx, pisObjArray[group]", t, r, n[t], " at sub2/pages/pis/pis.js:643"), r >= n[t].arr.length || "" != this.imgArrays[t][this.imgArrays[t].length - 1]) h.showPisCanvas = !1;
                                        else if ("" == this.imgArrays[t][r]) {
                                            this.loading = r;
                                            var c = uni.createCanvasContext("pisCanvas", h);
                                            c.rect(0, 0, 100, 100), c.setFillStyle("#1F2B38"), c.fill();
                                            var o = n[t].arr[r];
                                            this.drawObj(c, o, 50, 50, .1), c.draw(), this.$nextTick((function () {
                                                uni.canvasToTempFilePath({
                                                    canvasId: "pisCanvas",
                                                    success: function (e) {
                                                        var a = e.tempFilePath;
                                                        h.generatedImage = a, h.$set(h.imgArrays[t], r, a), h.createImg(t, ++r, n)
                                                    },
                                                    fail: function (e) { }
                                                })
                                            }))
                                        } else h.createImg(t, ++r, n)
                                }
                            },
                            pisCanvasDraw: function (e) {
                                this.showPisCanvas && this.createImg(this.pisSelectedGroup, 0, e)
                            },
                            doPisCanvasDraw: function (e) {
                                var t = this;
                                setTimeout((function () {
                                    var r = uni.createSelectorQuery().in(t);
                                    r.select("#imageListView").boundingClientRect((function (r) {
                                        null != r ? (t.imageListViewHeight = r.height - 30, setTimeout((function () {
                                            t.pisCanvasDraw(e)
                                        }), 10)) : t.doPisCanvasDraw(e)
                                    })).exec()
                                }), 10)
                            },
                            groupBtnClick: function (e) {
                                if (this.pisSelectedGroup != e) {
                                    this.pisSelectedGroup = e, this.pisSelectedIdx = -1, this.pisSelectedGroup == this.pisObj.cnfValus[1] && (this.pisSelectedIdx = this.pisObj.cnfValus[0]), this.showPisCanvas = this.checkIfShowPis(e);
                                    var t = this.getPicArray(),
                                        r = t.picArray;
                                    this.doPisCanvasDraw(r)
                                }
                            },
                            checkIfShowPis: function (e) {
                                if (5 == e || 6 == e) return !1;
                                if (0 == this.imgArrays.length) return !0;
                                if (0 == this.imgArrays[e].length) return !0;
                                for (var t = 0; t < this.imgArrays[e].length; t++)
                                    if ("" == this.imgArrays[e][t]) return !0;
                                return !1
                            },
                            cancelBtnClick: function (e) {
                                this.popupShow = !1, this.pisCanvasClear(), this.$refs.popup.close(), this.myCanvasDraw(), this.sendCmd()
                            },
                            okBtnClick: function (e) {
                                this.popupShow = !1, this.pisCanvasClear(), this.$refs.popup.close();
                                var t = this.convertPicIdxTo255(this.pisSelectedGroup, this.pisSelectedIdx);
                                this.$set(this.pisObj.cnfValus, 0, t.idx), this.$set(this.pisObj.cnfValus, 1, t.group), this.refreshChDraw(), this.myCanvasDraw(), this.sendCmd()
                            },
                            convertPic255ToIdx: function (e, t) {
                                var r = 0,
                                    n = 0;
                                return this.features.ilda ? (r = Math.floor(e / 25), r > 9 && (r = 9), n = 0, 0 != r && 1 != r || (n = Math.floor(t / 4)), 2 != r && 3 != r || (n = Math.floor(t / 6)), 5 != r && 6 != r || (n = Math.floor(t / 5))) : (r = Math.floor(e / 25), n = 0, 0 != r && 1 != r && 5 != r && 6 != r || (n = Math.floor(t / 5)), 2 != r && 3 != r || (n = Math.floor(t / 10)), 4 == r && (n = t <= 99 ? Math.floor(t / 5) : t >= 120 && t <= 159 ? Math.floor((t - 120) / 5) + 70 : t >= 100 && t <= 104 ? 20 : t >= 105 && t <= 109 ? 22 : t >= 110 && t <= 114 ? 24 : t >= 115 && t <= 119 ? 30 : 77), 6 == r && n > 14 && (n = 14)), {
                                    igroup: r,
                                    iidx: n
                                }
                            },
                            convertPicIdxTo255: function (groupIndex, itemIndex) {
                                var groupValue = 0;
                                var itemValue = 0;
                                if (this.features.ilda) {
                                    groupValue = 25 * groupIndex;
                                    itemValue = 0;
                                    if (groupIndex === 0 || groupIndex === 1) {
                                        itemValue = 4 * itemIndex;
                                    }
                                    if (groupIndex === 2 || groupIndex === 3) {
                                        itemValue = 6 * itemIndex;
                                    }
                                    if (groupIndex === 5 || groupIndex === 6) {
                                        itemValue = 5 * itemIndex;
                                    }
                                } else {
                                    groupValue = 25 * groupIndex;
                                    itemValue = 0;
                                    if (groupIndex === 0 || groupIndex === 1 || groupIndex === 5 || groupIndex === 6) {
                                        itemValue = 5 * itemIndex;
                                    }
                                    if (groupIndex === 2 || groupIndex === 3) {
                                        itemValue = 10 * itemIndex;
                                    }
                                    if (groupIndex === 4) {
                                        if (itemIndex <= 19) {
                                            itemValue = 5 * itemIndex;
                                        } else if (itemIndex >= 70 && itemIndex <= 77) {
                                            itemValue = 5 * (itemIndex - 70) + 120;
                                        } else if (itemIndex === 20) {
                                            itemValue = 100;
                                        } else if (itemIndex === 22) {
                                            itemValue = 105;
                                        } else if (itemIndex === 24) {
                                            itemValue = 110;
                                        } else if (itemIndex === 30) {
                                            itemValue = 115;
                                        } else {
                                            itemValue = 160;
                                        }
                                    }
                                }
                                return {
                                    group: groupValue,
                                    idx: itemValue
                                };
                            },
                            chResetClick: function (e) {
                                if (null != this.defaultPis) {
                                    for (var t = 0; t < this.defaultPis.cnfValus.length; t++) this.$set(this.pisObj.cnfValus, t, this.defaultPis.cnfValus[t]);
                                    this.refreshChDraw(), this.myCanvasDraw(), this.sendCmd()
                                }
                            },
                            myCanvasClick: function (e) {
                                this.popupShow = !0;
                                var t = this.convertPic255ToIdx(this.pisObj.cnfValus[1], this.pisObj.cnfValus[0]);
                                this.pisSelectedIdx = t.iidx, this.pisSelectedGroup = t.igroup, this.pisSelectedGroup >= this.pisObjArray.length && (this.pisSelectedGroup = 0, this.pisSelectedIdx = -1), this.showPisCanvas = this.checkIfShowPis(this.pisSelectedGroup), this.$refs.popup.open("bottom"), this.myCanvasClear();
                                var r = this.getPicArray(),
                                    n = r.picArray;
                                this.doPisCanvasDraw(n)
                            },
                            sendTmpCmd: function (selectedGroup, selectedIndex) {
                                var selectedConfigValues = [];
                                Object.assign(selectedConfigValues, this.pisObj.cnfValus);
                                var picIdxTo255 = this.convertPicIdxTo255(selectedGroup, selectedIndex);
                                selectedConfigValues[0] = picIdxTo255.idx;
                                selectedConfigValues[1] = picIdxTo255.group;
                                var commandData = {
                                    playTime: this.pisObj.playTime,
                                    cnfValus: selectedConfigValues
                                };
                                var commandString = i.getPisCmdStr(this.pisIdx, commandData, {
                                    features: this.features
                                });
                                c.gosend(!0, commandString);
                            },
                            imgClick: function (e) {
                                this.pisSelectedIdx = e, this.sendTmpCmd(this.pisSelectedGroup, this.pisSelectedIdx)
                            }
                        }
                    };
                t.default = s
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "sceneManagerPageComponent": function(e, t, r) {
            "use strict";
            (function(e) {
                Object.defineProperty(t, "__esModule", {
                    value: !0
                }), t.default = void 0;
                var n = getApp(),
                    h = r("deviceCommandUtils "),
                    a = r("bleDeviceControlUtils "),
                    i = {
                        data: function() {
                            var e = n.globalData.getDeviceFeatures(),
                                t = 650 * n.globalData.screen_width_float;
                            return {
                                screen_width: n.globalData.screen_width_str,
                                rtl: n.globalData.rtl,
                                features: e,
                                ntitle: this.$t("\u573a\u666f\u7ba1\u7406"),
                                scrollTop: 0,
                                oldScrollTop: 0,
                                imgArrays: [],
                                pisList: [{
                                    playTime: 0,
                                    cnfValus: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                                }, {
                                    playTime: 1,
                                    cnfValus: [2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                                }, {
                                    playTime: 2,
                                    cnfValus: [3, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                                }],
                                position: {
                                    x: t,
                                    y: t
                                },
                                startPosition: {
                                    x: 0,
                                    y: 0
                                },
                                defaultPis: {
                                    playTime: 5,
                                    cnfValus: [0, 0, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                                }
                            }
                        },
                        computed: {},
                        onLoad: function(t) {
                            e("log", "onload pgs", " at sub2/pages/pgs/pgs.js:49");
                            t.tag;
                            var r = n.globalData.getCmdData("pgsData");
                            this.pisList = r.pisList, this.initData()
                        },
                        onReady: function() {},
                        onShow: function() {},
                        onHide: function() {},
                        methods: {
                            sendCmd: function() {
                                n.globalData.setCmdData("pgsData", {
                                    pisList: this.pisList
                                });
                                n.globalData.getCmdData("pgsData");
                                var e = h.getPisListCmdStr(n.globalData.cmd.pgsData.pisList, {
                                    features: this.features
                                });
                                a.gosend(!0, e)
                            },
                            initData: function() {
                                this.features.ilda && (this.defaultPis = {
                                    playTime: 5,
                                    cnfValus: [0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                                })
                            },
                            onInputBlur: function(t) {
                                var r = this,
                                    n = 0;
                                t.detail.value && (n = parseInt(t.detail.value));
                                var h = n;
                                n < 1 && (h = 1), n > 25 && (h = 25);
                                var a = t.currentTarget.dataset.tag;
                                e("log", "-----", n, h, this.pisList[a].playTime, " at sub2/pages/pgs/pgs.js:87"), this.$set(this.pisList[a], "playTime", n), setTimeout((function() {
                                    r.$set(r.pisList[a], "playTime", h), r.sendCmd()
                                }), 100)
                            },
                            timeInput: function(e) {},
                            scrollToBottom: function() {
                                var e = this;
                                uni.createSelectorQuery().in(this).select("#scroll_view").fields({
                                    size: !0,
                                    scrollOffset: !0,
                                    scrollHeight: !0
                                }).exec((function(t) {
                                    if (t[0]) {
                                        var r = t[0].scrollHeight;
                                        e.scrollTop = e.oldScrollTop, e.$nextTick((function() {
                                            e.scrollTop = r
                                        }))
                                    }
                                }))
                            },
                            onscroll: function(e) {
                                this.oldScrollTop = e.detail.scrollTop
                            },
                            oprNew: function(e) {
                                if (this.pisList.length >= 20) n.globalData.showModalTips(this.$t("\u6700\u591a20\u4e2a\u573a\u666f"), !1);
                                else {
                                    var t = {
                                        playTime: 0,
                                        cnfValus: []
                                    };
                                    t.playTime = this.defaultPis.playTime, Object.assign(t.cnfValus, this.defaultPis.cnfValus), this.pisList.push(t), this.scrollToBottom(), this.sendCmd()
                                }
                            },
                            oprEdit: function(t) {
                                var r = this.pisList[t],
                                    n = this;
                                uni.navigateTo({
                                    url: "/sub2/pages/pis/pis",
                                    events: {
                                        acceptDataFromOpenedPage: function(r) {
                                            e("log", "acceptDataFromOpenedPage", r, " at sub2/pages/pgs/pgs.js:160"), n.$set(n.pisList, t, r.pis), n.imgArrays = r.imgArrays, n.sendCmd()
                                        }
                                    },
                                    success: function(e) {
                                        e.eventChannel.emit("acceptDataFromOpenerPage", {
                                            pis: r,
                                            idx: t,
                                            imgArrays: n.imgArrays,
                                            defaultPis: n.defaultPis
                                        })
                                    }
                                })
                            },
                            onBtnSetTouchStart: function(e) {
                                this.startPosition.x = e.touches[0].clientX - this.position.x, this.startPosition.y = e.touches[0].clientY - this.position.y
                            },
                            onBtnSetTouchMove: function(e) {
                                this.position.x = e.touches[0].clientX - this.startPosition.x, this.position.y = e.touches[0].clientY - this.startPosition.y
                            },
                            onBtnSetTouchEnd: function() {},
                            onBtnSetClick: function(e) {
                                uni.navigateTo({
                                    url: "/pages/subset/subset"
                                })
                            },
                            oprDelete: function(t) {
                                e("log", "oprDelete", t, " at sub2/pages/pgs/pgs.js:193");
                                var r = this;
                                uni.showModal({
                                    title: this.$t("\u63d0\u793a"),
                                    content: this.$t("\u662f\u5426\u5220\u9664\u573a\u666f") + t + "?",
                                    success: function(n) {
                                        n.confirm ? (r.pisList.splice(t, 1), r.sendCmd()) : n.cancel && e("log", "\u7528\u6237\u70b9\u51fb\u53d6\u6d88", " at sub2/pages/pgs/pgs.js:205")
                                    }
                                })
                            }
                        }
                    };
                t.default = i
            }).call(this, r("enhancedConsoleLogger")["default"])
        },


        "handDrawGeometryUtils": function (e, t, r) {
            (function (t) {
                var spreadToArrayHelper = r("spreadToArrayHelper"),
                    handDrawFileManager = r("handDrawFileManager"),
                    fontGeometryUtils = r("fontGeometryUtils "),
                    colors = ["black", "red", "green", "blue", "yellow", "#00FFFF", "purple", "white"];

                function rotatePointAroundCenter(e, t, r, n, h) {
                    var a = n - t,
                        i = h - r,
                        c = t + (a * Math.cos(e) - i * Math.sin(e)),
                        o = r + (a * Math.sin(e) + i * Math.cos(e));
                    return {
                        x: c,
                        y: o
                    }
                }

                function rotatePointsAroundBoundingBoxCenter(e, t) {
                    for (var r = arguments.length > 2 && void 0 !== arguments[2] && arguments[2], n = [], h = r ? 1 : -1, a = {
                        left: 99999,
                        top: 99999,
                        right: -99999,
                        bottom: -99999
                    }, i = 0; i < e.length; i++) {
                        var o = [e[i][0], h * e[i][1]];
                        a.left = Math.min(a.left, o[0]), a.top = Math.min(a.top, o[1]), a.right = Math.max(a.right, o[0]), a.bottom = Math.max(a.bottom, o[1])
                    }
                    for (var s = (a.right - a.left) / 2 + a.left, l = (a.bottom - a.top) / 2 + a.top, p = 0; p < e.length; p++) {
                        var d = e[p],
                            b = rotatePointAroundCenter(t, s, l, d[0], h * d[1]);
                        n.push([b.x, h * b.y, d[2], d[3]])
                    }
                    return n
                }

                function rotateGroupsOfPoints(e, t) {
                    for (var r = [], n = 0; n < e.length; n++) {
                        var h = e[n],
                            a = rotatePointsAroundBoundingBoxCenter(h, t, !0);
                        r.push(a)
                    }
                    return r
                }

                function calculateRotationOffset(e, t, r, n) {
                    r = -r;
                    var h = t + e.w,
                        a = r + e.h,
                        i = rotatePointAroundCenter(n, t, r, h, a),
                        o = {
                            mx: i.x - h,
                            my: i.y - a
                        };
                    return o
                }

                function rotateAndOffsetGroupedPoints(e) {
                    for (var t = e.ps, r = e.x0, n = e.y0, h = e.ang, a = [], i = {
                        left: 99999,
                        top: 99999,
                        right: -99999,
                        bottom: -99999
                    }, o = 0, s = 0; s < t.length; s++) {
                        var p = t[s][1];
                        if (o != t[s][0]) {
                            o = t[s][0], i["w"] = (i.right - i.left) / 2 + i.left, i["h"] = (i.bottom - i.top) / 2 + i.top;
                            var d = calculateRotationOffset(i, r, n, h);
                            i["mx"] = d.mx, i["my"] = d.my, a.push(i), i = {
                                left: 99999,
                                top: 99999,
                                right: -99999,
                                bottom: -99999
                            }
                        }
                        for (var b = 0; b < p.length; b++) {
                            var g = [p[b].x, -p[b].y];
                            i.left = Math.min(i.left, g[0]), i.top = Math.min(i.top, g[1]), i.right = Math.max(i.right, g[0]), i.bottom = Math.max(i.bottom, g[1])
                        }
                        if (s == t.length - 1) {
                            i["w"] = (i.right - i.left) / 2 + i.left, i["h"] = (i.bottom - i.top) / 2 + i.top;
                            var j = calculateRotationOffset(i, r, n, h);
                            i["mx"] = j.mx, i["my"] = j.my, a.push(i)
                        }
                    }
                    for (var x = [], V = 0; V < t.length; V++) {
                        for (var f = t[V][1], F = [], k = a[t[V][0]], m = 0; m < f.length; m++) {
                            var P = f[m],
                                u = rotatePointAroundCenter(h, k.w, k.h, P.x, -P.y);
                            F.push({
                                x: u.x + k.mx,
                                y: -(u.y + k.my),
                                z: P.z
                            })
                        }
                        x.push([t[V][0], F, t[V][2], t[V][3]])
                    }
                    return x
                }

                function getPointCount(e, t) {
                    var r = arguments.length > 2 && void 0 !== arguments[2] && arguments[2];
                    if (-1 == e) {
                        if (r) return t.length;
                        for (var n = 0, h = 0; h < t.length; h++) n += t[h].length;
                        return n
                    }
                    if (9999 == e) {
                        for (var a = 0, i = 0; i < t.length; i++) a += t[i][1].length;
                        return a
                    }
                    return t.length
                }

                function getAdjustedRectangle(e, t) {
                    var r = t,
                        n = e.left + e.mx - r,
                        h = e.top + e.my - r,
                        a = e.width * e.z + 2 * r,
                        i = e.height * e.z + 2 * r;
                    return {
                        left: n,
                        top: h,
                        width: a,
                        height: i
                    }
                }

                function drawTransformedPolyline(drawContext, drawObject, index) {
                    var accumulateResult = arguments.length > 3 && void 0 !== arguments[3] && arguments[3],
                        dashedLine = arguments.length > 4 && void 0 !== arguments[4] && arguments[4],
                        a = rotatePointsAroundBoundingBoxCenter(drawObject.ps[index], drawObject.ang, !0),
                        c = drawContext.ctx,
                        s = drawObject.lineColor,
                        l = s >= 8 ? 1 : s,
                        p = s - 9,
                        d = null;
                    p >= 0 && (d = drawContext.colorSeg[p]);
                    var b = drawObject.x0,
                        g = drawObject.y0,
                        j = drawObject.z;
                    c.beginPath(), dashedLine ? c.setLineDash(drawContext.draw_line_type) : c.setLineDash([]), c.setStrokeStyle(colors[l]), c.setFillStyle(colors[l]);
                    for (var x = a, V = 800 / drawContext.w, f = drawContext.w / 2, F = [], k = 0; k < x.length; k++) {
                        var m = [x[k][0] * j + b, x[k][1] * j + g, x[k][2], x[k][3]];
                        if (0 == k) c.moveTo(m[0], m[1]);
                        else {
                            var P = [x[k - 1][0] * j + b, x[k - 1][1] * j + g];
                            P[0] == m[0] && P[1] == m[1] ? (c.arc(m[0], m[1], 1, 0, 2 * Math.PI), c.fill(), c.moveTo(m[0], m[1])) : c.lineTo(m[0], m[1])
                        }
                        if (s >= 8) {
                            if (p < 0) l += 1, l = l >= 8 ? 1 : l;
                            else {
                                var u = Math.floor(k * d.color.length / x.length);
                                l = d.color[u]
                            }
                            c.setStrokeStyle(colors[l]), c.setFillStyle(colors[l]), c.stroke(), c.beginPath(), c.moveTo(m[0], m[1])
                        }
                        accumulateResult && F.push([(m[0] - f) * V, (f - m[1]) * V, 0 == k ? 0 : l, m[3]])
                    }
                    return c.stroke(), F
                }

                function drawTransformedPolyline2(drawObject, index, width) {

                    var a = rotatePointsAroundBoundingBoxCenter(drawObject.ps[index], drawObject.ang, !0),
                        s = drawObject.lineColor,
                        l = s >= 8 ? 1 : s,
                        p = s - 9,
                        d = null;
                    var b = drawObject.x0,
                        g = drawObject.y0,
                        j = drawObject.z;
                    for (var x = a, V = 800 / width, f = width / 2, F = [], k = 0; k < x.length; k++) {
                        var m = [x[k][0] * j + b, x[k][1] * j + g, x[k][2], x[k][3]];
                        if (s >= 8) {
                            if (p < 0) l += 1, l = l >= 8 ? 1 : l;
                            else if (d && d.color) {
                                var u = Math.floor(k * d.color.length / x.length);
                                l = d.color[u]
                            }
                        }
                        F.push([(m[0] - f) * V, (f - m[1]) * V, 0 == k ? 0 : l, m[3]])
                    }
                    return F
                }

                function drawAllTransformedPolylines(drawConfig, drawObject) {
                    for (var shouldCollectResults = arguments.length > 2 && void 0 !== arguments[2]
                        && arguments[2],
                        useDashedLine = arguments.length > 3 && void 0 !== arguments[3]
                            && arguments[3], points = drawObject.ps,
                        accumulatedResults = [], i = 0; i < points.length; i++) {
                        var currentPolylineResult = drawTransformedPolyline(drawConfig, drawObject, i, shouldCollectResults, useDashedLine);
                        shouldCollectResults && (accumulatedResults = accumulatedResults.concat(currentPolylineResult))
                    }
                    return accumulatedResults
                }

                function drawAllTransformedPolylines2(drawObject, width) {
                    for (var points = drawObject.ps,
                        accumulatedResults = [], i = 0; i < points.length; i++) {
                        var currentPolylineResult = drawTransformedPolyline2(drawObject, i, width);
                        (accumulatedResults = accumulatedResults.concat(currentPolylineResult))
                    }
                    return accumulatedResults

                }

                function drawTransformedObject(drawContext, drawObject) {
                    var accumulateResult = arguments.length > 2 && void 0 !== arguments[2] && arguments[2],
                        useDashedLine = arguments.length > 3 && void 0 !== arguments[3] && arguments[3],
                        rotatedPoints = rotatePointsAroundBoundingBoxCenter(drawObject.ps, drawObject.ang),
                        resultPoints = [],
                        scalingFactor = 800 / drawContext.w,
                        centerOffsetX = drawContext.w / 2,
                        ctx = drawContext.ctx,
                        positionX = drawObject.x0,
                        positionY = drawObject.y0,
                        scaleZ = drawObject.z;
                    ctx.beginPath(), useDashedLine ? ctx.setLineDash(drawContext.draw_line_type) : ctx.setLineDash([]);
                    var baseLineColor = drawObject.lineColor,
                        colorSegmentIndex = baseLineColor - 9,
                        currentColorIndex = baseLineColor >= 8 ? -1 : baseLineColor,
                        colorSegment = null;
                    colorSegmentIndex >= 0 && (colorSegment = drawContext.colorSeg[colorSegmentIndex]);
                    for (var pointIndex = 0; pointIndex < rotatedPoints.length; pointIndex++) {
                        colorSegmentIndex < 0 ? (currentColorIndex = baseLineColor >= 8 ? currentColorIndex + 1 : currentColorIndex, currentColorIndex = currentColorIndex >= 8 ? 1 : currentColorIndex) : currentColorIndex = colorSegment.color[Math.floor(pointIndex * colorSegment.color.length / rotatedPoints.length)];
                        var transformedPoint = [];
                        Object.assign(transformedPoint, rotatedPoints[pointIndex]), transformedPoint[0] = transformedPoint[0] * scaleZ + positionX, transformedPoint[1] = -transformedPoint[1] * scaleZ + positionY, 0 != transformedPoint[2] && (transformedPoint[2] = colorSegmentIndex < 0 ? currentColorIndex : 0 == colorSegmentIndex ? transformedPoint[2] : currentColorIndex);
                        var nextPoint = null;
                        if (pointIndex < rotatedPoints.length - 1 && (nextPoint = [], Object.assign(nextPoint, rotatedPoints[pointIndex + 1]), nextPoint[2] = colorSegmentIndex < 0 ? currentColorIndex + 1 : 0 == colorSegmentIndex ? nextPoint[2] : colorSegment.color[Math.floor((pointIndex + 1) * colorSegment.color.length / rotatedPoints.length)]), accumulateResult && resultPoints.push([rotatedPoints[pointIndex][0] * scalingFactor * scaleZ + (positionX - centerOffsetX) * scalingFactor, rotatedPoints[pointIndex][1] * scaleZ * scalingFactor + (-positionY + centerOffsetX) * scalingFactor, 0 == pointIndex ? 0 : transformedPoint[2], transformedPoint[3]]), null != nextPoint && transformedPoint[2] != nextPoint[2]) {
                            var activeColor = 0 == transformedPoint[2] ? nextPoint[2] : transformedPoint[2];
                            ctx.setStrokeStyle(colors[activeColor]), 0 != transformedPoint[2] && ctx.lineTo(transformedPoint[0], transformedPoint[1]), ctx.stroke(), ctx.beginPath(), ctx.moveTo(transformedPoint[0], transformedPoint[1])
                        } else 0 == transformedPoint[2] ? ctx.moveTo(transformedPoint[0], transformedPoint[1]) : ctx.lineTo(transformedPoint[0], transformedPoint[1]), null == nextPoint && ctx.setStrokeStyle(colors[transformedPoint[2]])
                    }
                    ctx.stroke(), ctx.beginPath();
                    var previousPoint = null;
                    currentColorIndex = baseLineColor >= 8 ? -1 : baseLineColor;
                    for (var secondLoopIndex = 0; secondLoopIndex < rotatedPoints.length; secondLoopIndex++) {
                        colorSegmentIndex < 0 ? (currentColorIndex = baseLineColor >= 8 ? currentColorIndex + 1 : currentColorIndex, currentColorIndex = currentColorIndex >= 8 ? 1 : currentColorIndex) : currentColorIndex = colorSegment.color[Math.floor(secondLoopIndex * colorSegment.color.length / rotatedPoints.length)];
                        var currentTransformedPoint = [],
                            originalPoint = [];
                        Object.assign(currentTransformedPoint, rotatedPoints[secondLoopIndex]), Object.assign(originalPoint, rotatedPoints[secondLoopIndex]), currentTransformedPoint[0] = currentTransformedPoint[0] * scaleZ + positionX, currentTransformedPoint[1] = -currentTransformedPoint[1] * scaleZ + positionY, 0 != currentTransformedPoint[2] && (currentTransformedPoint[2] = colorSegmentIndex < 0 ? currentColorIndex : 0 == colorSegmentIndex ? currentTransformedPoint[2] : currentColorIndex);
                        var nextTransformedPoint = null;
                        secondLoopIndex < rotatedPoints.length - 1 && (nextTransformedPoint = [], Object.assign(nextTransformedPoint, rotatedPoints[secondLoopIndex + 1]), 0 != nextTransformedPoint[2] && (nextTransformedPoint[2] = colorSegmentIndex < 0 ? currentColorIndex + 1 : 0 == colorSegmentIndex ? nextTransformedPoint[2] : colorSegment.color[Math.floor((secondLoopIndex + 1) * colorSegment.color.length / rotatedPoints.length)])), null != nextTransformedPoint && currentTransformedPoint[2] != nextTransformedPoint[2] ? null != previousPoint && previousPoint[0] == originalPoint[0] && previousPoint[1] == originalPoint[1] && (ctx.setStrokeStyle(colors[currentTransformedPoint[2]]), ctx.setFillStyle(colors[currentTransformedPoint[2]]), ctx.moveTo(currentTransformedPoint[0], currentTransformedPoint[1]), ctx.arc(currentTransformedPoint[0], currentTransformedPoint[1], 1, 0, 2 * Math.PI), ctx.stroke(), ctx.fill(), ctx.beginPath()) : null != previousPoint && previousPoint[0] == originalPoint[0] && previousPoint[1] == originalPoint[1] && (ctx.setStrokeStyle(colors[currentTransformedPoint[2]]), ctx.setFillStyle(colors[currentTransformedPoint[2]]), ctx.moveTo(currentTransformedPoint[0], currentTransformedPoint[1]), ctx.arc(currentTransformedPoint[0], currentTransformedPoint[1], 1, 0, 2 * Math.PI), ctx.stroke()), previousPoint = originalPoint
                    }
                    return ctx.fill(), resultPoints
                }

                function drawTransformedObject2(drawObject, width) {
                    var rotatedPoints = rotatePointsAroundBoundingBoxCenter(drawObject.ps, drawObject.ang),
                        resultPoints = [],
                        scalingFactor = 800 / width,
                        centerOffsetX = width / 2,
                        positionX = drawObject.x0,
                        positionY = drawObject.y0,
                        scaleZ = drawObject.z;
                    var baseLineColor = drawObject.lineColor,
                        colorSegmentIndex = baseLineColor - 9,
                        currentColorIndex = baseLineColor >= 8 ? -1 : baseLineColor;
                    for (var pointIndex = 0; pointIndex < rotatedPoints.length; pointIndex++) {
                        colorSegmentIndex < 0 ? (currentColorIndex = baseLineColor >= 8 ? currentColorIndex + 1 : currentColorIndex, currentColorIndex = currentColorIndex >= 8 ? 1 : currentColorIndex) : currentColorIndex = 1;
                        var transformedPoint = [];
                        Object.assign(transformedPoint, rotatedPoints[pointIndex]), transformedPoint[0] = transformedPoint[0] * scaleZ + positionX, transformedPoint[1] = -transformedPoint[1] * scaleZ + positionY, 0 != transformedPoint[2] && (transformedPoint[2] = colorSegmentIndex < 0 ? currentColorIndex : 0 == colorSegmentIndex ? transformedPoint[2] : currentColorIndex);
                        resultPoints.push([rotatedPoints[pointIndex][0] * scalingFactor * scaleZ + (positionX - centerOffsetX) * scalingFactor, rotatedPoints[pointIndex][1] * scaleZ * scalingFactor + (-positionY + centerOffsetX) * scalingFactor, 0 == pointIndex ? 0 : transformedPoint[2], transformedPoint[3]])
                    }
                    return resultPoints
                }


                function drawTransformedText(drawContext, drawObject) {
                    var accumulateResult = arguments.length > 2 && void 0 !== arguments[2] && arguments[2],
                        useDashedLine = arguments.length > 3 && void 0 !== arguments[3] && arguments[3],
                        transformedTextGroups = rotateAndOffsetGroupedPoints(drawObject),
                        result = [],
                        baseLineColor = drawObject.lineColor,
                        colorArray = colors,
                        currentStrokeColor = "red",
                        lineIndex = -1,
                        positionX = drawObject.x0,
                        positionY = drawObject.y0,
                        scaleZ = drawObject.z,
                        scalingFactor = 800 / drawContext.w,
                        centerOffsetX = drawContext.w / 2,
                        ctx = drawContext.ctx;
                    useDashedLine ? ctx.setLineDash(drawContext.draw_line_type) : ctx.setLineDash([]);
                    var colorSegmentIndex = drawObject.lineColor - 9,
                        colorSegment = null;
                    colorSegmentIndex >= 0 && (colorSegment = drawContext.colorSeg[colorSegmentIndex]);
                    for (var lastGroupId = -1, totalGroups = 0, groupIndex = 0; groupIndex < transformedTextGroups.length; groupIndex++) lastGroupId != transformedTextGroups[groupIndex][0] && (totalGroups++, lastGroupId = transformedTextGroups[groupIndex][0]);
                    var currentColorIndex = 1;
                    lastGroupId = -1, lineIndex = -1;
                    for (var textGroupIndex = 0; textGroupIndex < transformedTextGroups.length; textGroupIndex++) {
                        var pointsInGroup = transformedTextGroups[textGroupIndex][1];
                        if (lastGroupId != transformedTextGroups[textGroupIndex][0]) {
                            if (ctx.beginPath(), lastGroupId = transformedTextGroups[textGroupIndex][0], pointsInGroup.length > 1 && lineIndex++, colorSegmentIndex < 0) currentColorIndex = baseLineColor <= 7 ? baseLineColor : lineIndex % 7 + 1;
                            else if (0 == colorSegmentIndex) currentColorIndex = lineIndex % 7 + 1;
                            else {
                                var segmentColorIndex = Math.floor(lineIndex * colorSegment.color.length / totalGroups);
                                currentColorIndex = colorSegment.color[segmentColorIndex]
                            }
                            currentStrokeColor = colorArray[currentColorIndex], ctx.setStrokeStyle(currentStrokeColor), ctx.setFillStyle(currentStrokeColor)
                        }
                        for (var pointIndex = 0; pointIndex < pointsInGroup.length; pointIndex++) {
                            var currentPoint = pointsInGroup[pointIndex],
                                canvasX = currentPoint.x * scaleZ + positionX,
                                canvasY = positionY - currentPoint.y * scaleZ;
                            accumulateResult && result.push([currentPoint.x * scalingFactor * scaleZ + (positionX - centerOffsetX) * scalingFactor, currentPoint.y * scaleZ * scalingFactor + (-positionY + centerOffsetX) * scalingFactor, 0 == pointIndex ? 0 : currentColorIndex, currentPoint.z]), 0 == pointIndex ? ctx.moveTo(canvasX, canvasY) : Math.abs(currentPoint.x - pointsInGroup[pointIndex - 1].x) < 1 && Math.abs(currentPoint.y - pointsInGroup[pointIndex - 1].y) < 1 ? (ctx.arc(canvasX, canvasY, 1, 0, 2 * Math.PI), ctx.moveTo(canvasX, canvasY)) : ctx.lineTo(canvasX, canvasY)
                        }
                        ctx.stroke()
                    }
                    return result
                }

                function drawTransformedText2(drawObject, width) {
                    var transformedTextGroups = rotateAndOffsetGroupedPoints(drawObject),
                        result = [],
                        baseLineColor = drawObject.lineColor,
                        lineIndex = -1,
                        positionX = drawObject.x0,
                        positionY = drawObject.y0,
                        scaleZ = drawObject.z,
                        scalingFactor = 800 / width,
                        centerOffsetX = width / 2;
                    var colorSegmentIndex = drawObject.lineColor - 9;
                    for (var lastGroupId = -1, totalGroups = 0, groupIndex = 0; groupIndex < transformedTextGroups.length; groupIndex++) lastGroupId != transformedTextGroups[groupIndex][0] && (totalGroups++, lastGroupId = transformedTextGroups[groupIndex][0]);
                    var currentColorIndex = 1;
                    lastGroupId = -1, lineIndex = -1;
                    for (var textGroupIndex = 0; textGroupIndex < transformedTextGroups.length; textGroupIndex++) {
                        var pointsInGroup = transformedTextGroups[textGroupIndex][1];
                        if (lastGroupId != transformedTextGroups[textGroupIndex][0]) {
                            lastGroupId = transformedTextGroups[textGroupIndex][0], pointsInGroup.length > 1 && lineIndex++, colorSegmentIndex < 0 ? currentColorIndex = baseLineColor <= 7 ? baseLineColor : lineIndex % 7 + 1 : 0 == colorSegmentIndex ? currentColorIndex = lineIndex % 7 + 1 : currentColorIndex = 1
                        }
                        for (var pointIndex = 0; pointIndex < pointsInGroup.length; pointIndex++) {
                            var currentPoint = pointsInGroup[pointIndex];
                            result.push([currentPoint.x * scalingFactor * scaleZ + (positionX - centerOffsetX) * scalingFactor, currentPoint.y * scaleZ * scalingFactor + (-positionY + centerOffsetX) * scalingFactor, 0 == pointIndex ? 0 : currentColorIndex, currentPoint.z])
                        }
                    }
                    return result
                }

                function resizeGroupedPoints(e, t, r, n, h) {
                    for (var a = e.ps, i = [], c = n / t, o = h / r, s = 0; s < a.length; s++) {
                        for (var l = a[s], p = [], d = 0; d < l.length; d++) {
                            var b = l[d];
                            p.push([b[0] * c, b[1] * o, b[2], b[3]])
                        }
                        i.push(p)
                    }
                    var g = e.x0 * c,
                        j = e.y0 * o,
                        x = {
                            ps: i,
                            x0: g,
                            y0: j,
                            z: e.z,
                            drawMode: e.drawMode,
                            ang: e.ang,
                            lineColor: e.lineColor
                        };
                    return x
                }

                function resizeStructuredGroupedPoints(e, t, r, n, h) {
                    for (var a = e.ps, i = [], c = n / t, o = h / r, s = 0; s < a.length; s++) {
                        for (var l = a[s][1], p = [], d = 0; d < l.length; d++) {
                            var b = l[d];
                            p.push({
                                x: b.x * c,
                                y: b.y * o,
                                z: b.z
                            })
                        }
                        i.push([a[s][0], p, a[s][2] * c, a[s][3] * o])
                    }
                    var g = e.x0 * c,
                        j = e.y0 * o,
                        x = {
                            ps: i,
                            x0: g,
                            y0: j,
                            z: e.z,
                            drawMode: e.drawMode,
                            ang: e.ang,
                            lineColor: e.lineColor
                        };
                    return x
                }

                function resizeFlatGroupedPoints(e, t, r, n, h) {
                    for (var a = e.ps, i = [], c = n / t, o = h / r, s = 0; s < a.length; s++) {
                        var l = a[s];
                        i.push([l[0] * c, l[1] * o, l[2], l[3]])
                    }
                    var p = e.x0 * c,
                        d = e.y0 * o,
                        b = {
                            ps: i,
                            x0: p,
                            y0: d,
                            z: e.z,
                            drawMode: e.drawMode,
                            ang: e.ang,
                            lineColor: e.lineColor
                        };
                    return b
                }
                e.exports = {
                    defaultWith: 300,
                    defaultHeight: 300,
                    colorSeg: [{
                        color: [1, 1, 1, 1, 1, 4, 4, 4, 4, 4, 2, 2, 2, 2, 5, 5, 5, 5, 5, 3, 3, 3, 3, 6, 6, 6, 6, 6, 7, 7, 7, 7],
                        name: "Rainbow (8 segments)"
                    }, {
                        color: [1, 1, 1, 1, 1, 4, 4, 4, 4, 4, 2, 2, 2, 2, 5, 5, 5, 5, 5, 3, 3, 3, 3, 6, 6, 6, 6, 6, 7, 7, 7, 7],
                        name: "Rainbow (8 segments)"
                    }, {
                        color: [3, 3, 3, 3, 3, 3, 3, 3, 7, 7, 7, 7, 7, 7, 7, 7, 2, 2, 2, 2, 2, 2, 2, 2, 7, 7, 7, 7, 7, 7, 7, 7],
                        name: "PICK_3 4 segments"
                    }, {
                        color: [4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 6, 6],
                        name: "PICK_4 4 segments"
                    }, {
                        color: [7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2, 7, 7, 7, 7, 2, 2, 2, 2],
                        name: "White-Green (8 segments)"
                    }, {
                        color: [3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7, 3, 3, 3, 3, 7, 7, 7, 7],
                        name: "White-Blue (8 segments)"
                    }, {
                        color: [7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1, 7, 7, 7, 7, 1, 1, 1, 1],
                        name: "White-Red (8 segments)"
                    }, {
                        color: [4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5, 4, 4, 4, 4, 5, 5, 5, 5],
                        name: "Green-Yellow (8 segments)"
                    }, {
                        color: [7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1, 7, 7, 1, 1],
                        name: "White-Red (16 segments)"
                    }, {
                        color: [6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5, 6, 6, 5, 5],
                        name: "Blue-Green (16 segments)"
                    }, {
                        color: [6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4, 6, 6, 4, 4],
                        name: "Yellow-Brown (16 segments)"
                    }, {
                        color: [4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3, 4, 4, 3, 3],
                        name: "Purple-Yellow (16 segments)"
                    }],
                    getPointCount: getPointCount,
                    getTextRect: function (e, t) {
                        txXy = rotateAndOffsetGroupedPoints(e);
                        for (var r = 0; r < txXy.length; r++)
                            for (var n = txXy[r][1], h = 0; h < n.length; h++) {
                                var a = [n[h].x * e.z + e.x0, e.y0 - n[h].y * e.z];
                                t.left = Math.min(t.left, a[0]), t.top = Math.min(t.top, a[1]), t.right = Math.max(t.right, a[0]), t.bottom = Math.max(t.bottom, a[1])
                            }
                        return t
                    },
                    getLineRect: function (e, t) {
                        for (var r = rotateGroupsOfPoints(e.ps, e.ang), n = 0; n < r.length; n++)
                            for (var h = r[n], a = 0; a < h.length; a++) {
                                var i = [h[a][0] * e.z + e.x0, e.y0 + h[a][1] * e.z];
                                t.left = Math.min(t.left, i[0]), t.top = Math.min(t.top, i[1]), t.right = Math.max(t.right, i[0]), t.bottom = Math.max(t.bottom, i[1])
                            }
                        return t
                    },
                    getObjRect: function (e, t) {
                        ps = rotatePointsAroundBoundingBoxCenter(e.ps, e.ang);
                        for (var r = 0; r < ps.length; r++) {
                            var n = [ps[r][0] * e.z + e.x0, e.y0 - ps[r][1] * e.z];
                            t.left = Math.min(t.left, n[0]), t.top = Math.min(t.top, n[1]), t.right = Math.max(t.right, n[0]), t.bottom = Math.max(t.bottom, n[1])
                        }
                        return t
                    },
                    checkObj: function (e, t, r, n, a) {
                        for (var i = 1; i < e.length; i++) {
                            var c = [e[i - 1][0] * n + t, r - e[i - 1][1] * n],
                                o = [e[i][0] * n + t, r - e[i][1] * n],
                                s = [c, o];
                            if (handDrawFileManager.lineCross(s, a)) return !0
                        }
                    },
                    checkText: function (e, t, r, n, a) {
                        for (var i = 0; i < e.length; i++)
                            for (var c = e[i][1], o = 1; o < c.length; o++) {
                                var s = [c[o - 1].x * n + t, r - c[o - 1].y * n],
                                    l = [c[o].x * n + t, r - c[o].y * n],
                                    p = [s, l];
                                if (handDrawFileManager.lineCross(p, a)) return !0
                            }
                    },
                    checkLine: function (e, t, r, n, a) {
                        for (var i = e, c = 0; c < i.length; c++)
                            for (var o = i[c], s = 1; s < o.length; s++) {
                                var l = [o[s - 1][0] * n + t, o[s - 1][1] * n + r],
                                    p = [o[s][0] * n + t, o[s][1] * n + r],
                                    d = [l, p];
                                if (handDrawFileManager.lineCross(d, a)) return !0
                            }
                        return !1
                    },
                    covertPoints: function (e, t, r) {
                        for (var n = fontGeometryUtils.parseLines(e, r), h = {
                            left: 99999,
                            top: 99999,
                            right: 0,
                            bottom: 0
                        }, i = 0; i < n.length; i++)
                            for (var c = n[i], o = 0; o < c.length; o++) {
                                var s = c[o];
                                h.left = Math.min(h.left, s[0]), h.top = Math.min(h.top, s[1]), h.right = Math.max(h.right, s[0]), h.bottom = Math.max(h.bottom, s[1])
                            }
                        for (var l = (h.right - h.left) / 2 + h.left, p = (h.bottom - h.top) / 2 + h.top, d = 0; d < n.length; d++)
                            for (var b = 0; b < n[d].length; b++) n[d][b][0] = n[d][b][0] - l, n[d][b][1] = n[d][b][1] - p;
                        return {
                            drawMode: -1,
                            ps: n,
                            x0: l,
                            y0: p,
                            z: 1,
                            lineColor: t
                        }
                    },
                    lineTheta: function (e, t, r) {
                        var n = {
                            x: e[0] - t[0],
                            y: e[1] - t[1]
                        },
                            h = {
                                x: r[0] - t[0],
                                y: r[1] - t[1]
                            },
                            a = n.x * h.x + n.y * h.y,
                            i = Math.sqrt(Math.pow(n.x, 2) + Math.pow(n.y, 2)),
                            c = Math.sqrt(Math.pow(h.x, 2) + Math.pow(h.y, 2)),
                            o = Math.acos(a / (i * c));
                        return o
                    },
                    getTextLineSize: function (e, t) {
                        for (var r = 0, n = 0, h = -1, a = 0; a < e.length; a++) {
                            var i = e[a];
                            h != i[0] && (t ? (r += i[2], n = i[3]) : (r = i[3], n += i[3]), h = i[0])
                        }
                        return {
                            w: r,
                            h: n
                        }
                    },
                    calcAngle: function (e, t, r, n) {
                        var h = Math.atan2(t - n, e - r);
                        return h
                    },
                    calcAngXY: rotatePointAroundCenter,
                    calcObjAngXY: rotatePointsAroundBoundingBoxCenter,
                    calcLinesAngXY: rotateGroupsOfPoints,
                    pointInRectangle: function (e, t, r, h, a, i) {
                        var c = r.x,
                            o = h.x,
                            s = a.x,
                            l = i.x,
                            p = r.y,
                            d = h.y,
                            b = a.y,
                            g = i.y;
                        if (e >= c && e <= o && t >= p && t <= b) return !0;
                        if (e === c && t === p || e === o && t === d || e === s && t === b || e === l && t === g) return !0;
                        for (var j = 0, x = [
                            [c, p, o, d],
                            [o, d, s, b],
                            [s, b, l, g],
                            [l, g, c, p]
                        ], V = 0; V < x.length; V++) {
                            var f = spreadToArrayHelper(x[V], 4),
                                F = f[0],
                                k = f[1],
                                m = f[2],
                                P = f[3];
                            if (t > Math.min(k, P) && t <= Math.max(k, P) && e <= Math.max(F, m) && k !== P) {
                                var u = (t - k) * (m - F) / (P - k) + F;
                                (F === m || e <= u) && j++
                            }
                        }
                        return j % 2 === 1
                    },
                    getSelectRectInfo: function (e) {
                        var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 2,
                            r = getAdjustedRectangle(e, t),
                            n = r.left,
                            h = r.top,
                            a = r.width,
                            i = r.height,
                            o = n + a / 2,
                            s = h + i / 2,
                            l = e.lastAng - e.startAng + e.ang,
                            p = rotatePointAroundCenter(l, o, s, n, h),
                            d = rotatePointAroundCenter(l, o, s, n + a, h),
                            g = rotatePointAroundCenter(l, o, s, n + a, h + i),
                            j = rotatePointAroundCenter(l, o, s, n, h + i),
                            x = {
                                x0: o,
                                y0: s,
                                p1: p,
                                p2: d,
                                p3: g,
                                p4: j
                            };
                        return x
                    },
                    getUiRectSize: function (e) {
                        for (var t = [
                            [e.p1.x, e.p1.y],
                            [e.p2.x, e.p2.y],
                            [e.p3.x, e.p3.y],
                            [e.p4.x, e.p4.y]
                        ], r = {
                            left: 99999,
                            top: 99999,
                            right: -99999,
                            bottom: -99999,
                            width: 0,
                            height: 0
                        }, n = 0; n < t.length; n++) {
                            var h = [t[n][0], t[n][1]];
                            r.left = Math.min(r.left, h[0]), r.top = Math.min(r.top, h[1]), r.right = Math.max(r.right, h[0]), r.bottom = Math.max(r.bottom, h[1])
                        }
                        return r.width = r.right - r.left, r.height = r.bottom - r.top, r
                    },
                    getCenterCorss: function (e, t, r) {
                        var n = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : 20,
                            h = n / 2,
                            a = n / 5,
                            i = rotatePointAroundCenter(e, t, r, t - h, r),
                            o = rotatePointAroundCenter(e, t, r, t + h, r),
                            s = rotatePointAroundCenter(e, t, r, t - h + a, r - a),
                            l = rotatePointAroundCenter(e, t, r, t - h + a, r + a),
                            p = rotatePointAroundCenter(e, t, r, t + h - a, r - a),
                            d = rotatePointAroundCenter(e, t, r, t + h - a, r + a),
                            b = rotatePointAroundCenter(e, t, r, t, r - h),
                            g = rotatePointAroundCenter(e, t, r, t, r + h),
                            j = rotatePointAroundCenter(e, t, r, t - a, r - h + a),
                            x = rotatePointAroundCenter(e, t, r, t + a, r - h + a),
                            V = rotatePointAroundCenter(e, t, r, t - a, r + h - a),
                            f = rotatePointAroundCenter(e, t, r, t + a, r + h - a),
                            F = {
                                p1: i,
                                p2: o,
                                p3: b,
                                p4: g,
                                p11: s,
                                p12: l,
                                p21: p,
                                p22: d,
                                p31: j,
                                p32: x,
                                p41: V,
                                p42: f
                            };
                        return F
                    },
                    calcTextAngXY: rotateAndOffsetGroupedPoints,
                    drawText: drawTransformedText,
                    drawObj: drawTransformedObject,
                    drawLine: drawAllTransformedPolylines,
                    drawPs: function (objectsToDraw, drawConfig, selectionState) {
                        var n = [],
                            drawContext = drawConfig.ctx;
                        drawContext.clearRect(0, 0, drawConfig.w, drawConfig.h);
                        for (var a = [], i = 0; i < objectsToDraw.length; i++) {
                            var o = objectsToDraw[i],
                                s = !1;
                            if (selectionState && (s = selectionState.selectLines.length > i && selectionState.selectLines[i].sel & selectionState.selectMode, s && null != selectionState.selectRect)) {
                                var l = selectionState.selectRect.lastAng - selectionState.selectRect.startAng;
                                1 != selectionState.selectRect.z ? (selectionState.selectLines[i].mx0 = selectionState.selectLines[i].mx0 * selectionState.selectRect.z, selectionState.selectLines[i].my0 = selectionState.selectLines[i].my0 * selectionState.selectRect.z, o.x0 = selectionState.selectLines[i].mx0 + selectionState.selectRect.x0, o.y0 = selectionState.selectLines[i].my0 + selectionState.selectRect.y0, l = selectionState.selectRect.lastAng - selectionState.selectRect.startAng + selectionState.selectRect.ang, o.z = o.z * selectionState.selectRect.z) : (o.x0 = o.x0 + selectionState.selectRect.mx + selectionState.selectRect.width * (selectionState.selectRect.z - 1) * .5, o.y0 = o.y0 + selectionState.selectRect.my + selectionState.selectRect.height * (selectionState.selectRect.z - 1) * .5), null != selectionState.selectLines[i].color && (o.lineColor = selectionState.selectLines[i].color);
                                var p = rotatePointAroundCenter(l, selectionState.selectRect.x0, selectionState.selectRect.y0, o.x0, o.y0);
                                o.x0 = p.x, o.y0 = p.y, o.ang = selectionState.selectRect.lastAng - selectionState.selectRect.startAng + o.ang
                            }
                            a = -1 == o.drawMode ? drawAllTransformedPolylines(drawConfig, o, !0, s) : 9999 == o.drawMode ? drawTransformedText(drawConfig, o, !0, s) : drawTransformedObject(drawConfig, o, !0, s), n = n.concat(a)
                        }
                        return selectionState && null != selectionState.selectRect && (selectionState.selectRect.left = selectionState.selectRect.left + selectionState.selectRect.mx, selectionState.selectRect.top = selectionState.selectRect.top + selectionState.selectRect.my, selectionState.selectRect.width = selectionState.selectRect.width * selectionState.selectRect.z, selectionState.selectRect.height = selectionState.selectRect.height * selectionState.selectRect.z, selectionState.selectRect.z = 1, selectionState.selectRect.mx = 0, selectionState.selectRect.my = 0), n
                    },
                    drawPs2: function (objectsToDraw, width) {
                        var points = [];
                        for (var currentDrawResult = [], i = 0; i < objectsToDraw.length; i++) {
                            var drawObject = objectsToDraw[i];
                            currentDrawResult = -1 == drawObject.drawMode
                                ? drawAllTransformedPolylines2(drawObject, width)
                                : 9999 == drawObject.drawMode
                                    ? drawTransformedText2(drawObject, width)
                                    : drawTransformedObject2(drawObject, width),
                                points = points.concat(currentDrawResult)
                        }
                        return points
                    },
                    getdrawPointsCnt: function (e) {
                        for (var t = 0, r = 0; r < e.length; r++) t += getPointCount(e[r].drawMode, e[r].ps);
                        return t
                    },
                    reSizeDrawPoints: function (e, r, n) {
                        var h = arguments.length > 3 && void 0 !== arguments[3] ? arguments[3] : 300,
                            a = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : 300,
                            i = [];
                        t("log", "orgWidth, orgHeight, destWidth, destHeight", r, n, h, a, " at sub/pages/utils/drawFunc.js:865");
                        for (var c = 0; c < e.length; c++) {
                            var o = e[c],
                                s = [];
                            s = -1 == o.drawMode ? resizeGroupedPoints(o, r, n, h, a) : 9999 == o.drawMode ? resizeStructuredGroupedPoints(o, r, n, h, a) : resizeFlatGroupedPoints(o, r, n, h, a), i.push(s)
                        }
                        return i
                    }
                }
            }).call(this, r("enhancedConsoleLogger")["default"])
        },

        "handDrawFileManager": function (t, r, n) {
            (function (r) {
                var spreadToArrayHelper = n("spreadToArrayHelper");

                function createIterator(e, t) {
                    var r = "undefined" !== typeof Symbol && e[Symbol.iterator] || e["@@iterator"];
                    if (!r) {
                        if (Array.isArray(e) || (r = function (e, t) {
                            if (!e) return;
                            if ("string" === typeof e) return i(e, t);
                            var r = Object.prototype.toString.call(e).slice(8, -1);
                            "Object" === r && e.constructor && (r = e.constructor.name);
                            if ("Map" === r || "Set" === r) return Array.from(e);
                            if ("Arguments" === r || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(r)) return i(e, t)
                        }(e)) || t && e && "number" === typeof e.length) {
                            r && (e = r);
                            var n = 0,
                                h = function () { };
                            return {
                                s: h,
                                n: function () {
                                    return n >= e.length ? {
                                        done: !0
                                    } : {
                                        done: !1,
                                        value: e[n++]
                                    }
                                },
                                e: function (e) {
                                    throw e
                                },
                                f: h
                            }
                        }
                        throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.")
                    }
                    var a, c = !0,
                        o = !1;
                    return {
                        s: function () {
                            r = r.call(e)
                        },
                        n: function () {
                            var e = r.next();
                            return c = e.done, e
                        },
                        e: function (e) {
                            o = !0, a = e
                        },
                        f: function () {
                            try {
                                c || null == r.return || r.return()
                            } finally {
                                if (o) throw a
                            }
                        }
                    }
                }

                function i(e, t) {
                    (null == t || t > e.length) && (t = e.length);
                    for (var r = 0, n = new Array(t); r < t; r++) n[r] = e[r];
                    return n
                }
                var c = "handdrawtag_",
                    o = ["ALL", "Group 1", "Group 2", "Group 3", "Group 4", "Group 5", "Group 6", "Group 7", "Group 8", "Group 9", "Group 10"],
                    s = 0;

                function l(e) {
                    for (var t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : 2, r = e + "", n = 0; n < t; n++) {
                        if (r.length >= t) break;
                        r = "0" + r
                    }
                    return r
                }

                function p(e) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] && arguments[1];
                    return d(c, e, t)
                }

                function d(t, n) {
                    var h = arguments.length > 2 && void 0 !== arguments[2] && arguments[2];
                    try {
                        h || (n = t + n), n = n.toLowerCase();
                        var a = uni.getStorageSync(n);
                        return a
                    } catch (e) {
                        r("log", "getFromFile fail:", JSON.stringify(e), " at utils/funCtrl.js:187")
                    }
                    return null
                }

                function b(t, n, h) {
                    var a = arguments.length > 3 && void 0 !== arguments[3] && arguments[3],
                        i = V(t);
                    a || (n = t + n), n = n.toLowerCase();
                    var c = i.indexOf(n); - 1 == c && i.unshift(n);
                    try {
                        var o = x(t, i);
                        return !!o && (uni.setStorageSync(n, h), !0)
                    } catch (e) {
                        r("log", "saveToFile fail:", JSON.stringify(e), " at utils/funCtrl.js:204")
                    }
                    return !1
                }

                function g(t, n, h) {
                    var a = arguments.length > 3 && void 0 !== arguments[3] && arguments[3];
                    try {
                        var i = d(t, n, a);
                        return null == i || !j(t, n, a) || b(t, h, i, a)
                    } catch (e) {
                        r("log", "reNameFile fail:", JSON.stringify(e), " at utils/funCtrl.js:216")
                    }
                    return !1
                }

                function j(t, n) {
                    var h = arguments.length > 2 && void 0 !== arguments[2] && arguments[2];
                    try {
                        var a = !0;
                        r("log", "fileTag, fileName", t, n, " at utils/funCtrl.js:224"), h || (n = t + n), n = n.toLowerCase(), uni.removeStorageSync(n);
                        var i = V(t),
                            c = i.indexOf(n);
                        return -1 != c && (i.splice(c, 1), a = x(t, i)), a
                    } catch (e) {
                        r("log", "deleteFile fail:", JSON.stringify(e), " at utils/funCtrl.js:236")
                    }
                    return !1
                }

                function x(t, n) {
                    try {
                        var h = "fileTag_" + t;
                        return uni.setStorageSync(h, n), !0
                    } catch (e) {
                        r("log", "setSaveFilenames fail:", JSON.stringify(e), " at utils/funCtrl.js:247")
                    }
                    return !1
                }

                function V(t) {
                    var n = arguments.length > 1 && void 0 !== arguments[1] && arguments[1];
                    try {
                        var h = "fileTag_" + t,
                            i = uni.getStorageSync(h);
                        if (r("log", "getSaveFileNames", h, i, " at utils/funCtrl.js:256"), i) {
                            if (n) {
                                var c, o = [],
                                    s = createIterator(i);
                                try {
                                    for (s.s(); !(c = s.n()).done;) {
                                        var l = c.value;
                                        0 == l.indexOf(t) && o.push(l.replace(t, ""))
                                    }
                                } catch (p) {
                                    s.e(p)
                                } finally {
                                    s.f()
                                }
                                return o
                            }
                            return i
                        }
                    } catch (e) {
                        r("log", "getSaveFileNames fail:", JSON.stringify(e), " at utils/funCtrl.js:270")
                    }
                    return []
                }

                function f(e) {
                    var t = e.split("_split_tag_"),
                        r = {
                            name: "",
                            class: ""
                        };
                    return 2 == t.length ? (r["name"] = t[0], r["class"] = t[1]) : r["name"] = t[0], r
                }

                function F(e) {
                    var t = arguments.length > 1 && void 0 !== arguments[1] && arguments[1];
                    return d("playlistfiletag_", e, t)
                }

                function k() {
                    var e = V("playlistfiletag_", !0),
                        t = {
                            fileNames: e,
                            noSpace: !1,
                            count: e.length
                        };
                    return t
                }
                t.exports = {
                    handDrawClassFix: ["__ALL__", "__1__", "__2__", "__3__", "__4__", "__5__", "__6__", "__7__", "__8__", "__9__", "__10__"],
                    savePlayListFileData: function (e, t) {
                        var r = arguments.length > 2 && void 0 !== arguments[2] && arguments[2],
                            n = {
                                data: t
                            };
                        return b("playlistfiletag_", e, n, r)
                    },
                    getPlayListFileData: F,
                    getPlayListFileNames: k,
                    getNewPlayListName: function () {
                        var e = new Date,
                            t = e.getFullYear(),
                            r = e.getMonth() + 1,
                            n = e.getDate(),
                            h = "list_" + l(t) + l(r) + l(n) + "_",
                            a = 0;
                        while (1) {
                            a++;
                            var i = h + ("00" + a).slice(-2);
                            if (value = uni.getStorageSync("playlistfiletag_" + i), !value) return i
                        }
                        return h
                    },
                    deletePlayList: function (e) {
                        return j("playlistfiletag_", e)
                    },
                    getIncludePlayList: function (e) {
                        var t = k();
                        r("log", "rs", JSON.stringify(t), " at utils/funCtrl.js:397");
                        var n, h = t.fileNames,
                            i = [],
                            c = createIterator(h);
                        try {
                            for (c.s(); !(n = c.n()).done;) {
                                var o = n.value,
                                    s = F(o);
                                if (s) {
                                    var l, p = createIterator(s.data);
                                    try {
                                        for (p.s(); !(l = p.n()).done;) {
                                            var d = l.value;
                                            if (d.fileName == e) {
                                                i.push(o);
                                                break
                                            }
                                        }
                                    } catch (b) {
                                        p.e(b)
                                    } finally {
                                        p.f()
                                    }
                                }
                            }
                        } catch (b) {
                            c.e(b)
                        } finally {
                            c.f()
                        }
                        return i
                    },
                    drawPointsHisCount: s,
                    getFileClass: function (e) {
                        var t, r = [],
                            n = createIterator(e);
                        try {
                            for (n.s(); !(t = n.n()).done;) {
                                var h = t.value,
                                    i = f(h),
                                    c = i.class;
                                "" != c && -1 == r.indexOf(c) && r.push(c)
                            }
                        } catch (o) {
                            n.e(o)
                        } finally {
                            n.f()
                        }
                        return r.sort((function (e, t) {
                            return t.localeCompare(e)
                        })), r.unshift("__ALL__"), r
                    },
                    splitFileClass: f,
                    combiFileName: function (e, t) {
                        return "__ALL__" == e ? t : t + "_split_tag_" + e
                    },
                    getDrawPointsHisCount: function () {
                        return s
                    },
                    updateHandDrawImgPlayTime: function (e, t) {
                        var r = p(e);
                        r.pisObj.cnfValus[12] = t, b(c, e, r, !1)
                    },
                    fileClassSplitTag: "_split_tag_",
                    handDrawTag: c,
                    getTextFileNames: function () {
                        var e = V("textfiletag_", !0),
                            t = {
                                fileNames: e,
                                noSpace: !1,
                                count: e.length
                            };
                        return t
                    },
                    getTextFileData: function (e) {
                        var t = arguments.length > 1 && void 0 !== arguments[1] && arguments[1];
                        return d("textfiletag_", e, t)
                    },
                    separateValueAndUnit: function (e) {
                        var t = {
                            value: null,
                            unit: null
                        },
                            r = e.match(/^(\d+)([a-zA-Z]+)?/);
                        return r && (t.value = parseInt(r[1]), t.unit = r[2] || ""), t
                    },
                    reNameHandDrawImg: function (e, t) {
                        return g(c, e, t)
                    },
                    pushDrawPointsHis: function (e) {
                        r("log", "pushDrawPointsHis", e, " at utils/funCtrl.js:276");
                        var t = "drawpointshistag_" + s,
                            n = {
                                data: e,
                                idx: s
                            };
                        r("log", "fileKey", t, " at utils/funCtrl.js:280"), uni.setStorageSync(t.toLowerCase(), n), s += 1
                    },
                    lineCross: function (e, t) {
                        function r(e, t, r, n) {
                            var h = (r[1] - e[1]) * (e[0] - t[0]) - (r[0] - e[0]) * (e[1] - t[1]),
                                a = (n[1] - e[1]) * (e[0] - t[0]) - (n[0] - e[0]) * (e[1] - t[1]);
                            return !(h * a > 0)
                        }
                        var n = spreadToArrayHelper(e, 2),
                            a = n[0],
                            i = n[1],
                            c = spreadToArrayHelper(t, 2),
                            o = c[0],
                            s = c[1];
                        return !!r(a, i, o, s) && !!r(o, s, a, i)
                    },
                    deleteHandDrawImg: function (e) {
                        return j(c, e)
                    },
                    saveTextFileData: function (e, t, r) {
                        var n = arguments.length > 3 && void 0 !== arguments[3] && arguments[3],
                            h = {
                                data: t,
                                dataSize: r
                            };
                        return b("textfiletag_", e, h, n)
                    },
                    getDistance: function (e) {
                        var t = e[1].x - e[0].x,
                            r = e[1].y - e[0].y;
                        return Math.sqrt(t * t + r * r)
                    },
                    getHandDrawImg: p,
                    deleteTextFileData: function (e) {
                        return j("textfiletag_", e)
                    },
                    getNewFileName: function () {
                        var e = !(arguments.length > 0 && void 0 !== arguments[0]) || arguments[0],
                            t = new Date,
                            r = t.getFullYear(),
                            n = t.getMonth() + 1,
                            h = t.getDate(),
                            a = "file_" + l(r) + l(n) + l(h) + "_",
                            i = "textfiletag_";
                        e && (i = c);
                        var o = 0;
                        while (1) {
                            o++;
                            var s = a + ("00" + o).slice(-2);
                            if (value = uni.getStorageSync(i + s), !value) return s
                        }
                        return a
                    },
                    isImgFileExist: function (e, t) {
                        if (!e) return r("log", "isImgFileExist picPath=", e, " at utils/funCtrl.js:88"), void t(!1);
                        uni.getImageInfo({
                            src: e,
                            success: function (e) {
                                t(!0)
                            },
                            fail: function (e) {
                                t(!1)
                            }
                        })
                    },
                    reNameTextFile: function (e, t) {
                        return g("textfiletag_", e, t)
                    },
                    popDrawPointsHis: function () {
                        s--;
                        var t = "drawpointshistag_" + s;
                        r("log", "popDrawPointsHis", t, " at utils/funCtrl.js:288");
                        var n = uni.getStorageSync(t);
                        try {
                            uni.removeStorageSync(t)
                        } catch (e) { }
                        return n
                    },
                    saveHandDrawImg: function (e, t, r, n, h, a) {
                        var i = arguments.length > 6 && void 0 !== arguments[6] && arguments[6],
                            o = {
                                picPath: t,
                                drawPoints: r,
                                pointCnt: n,
                                pisObj: h,
                                features: a
                            };
                        return b(c, e, o, i)
                    },
                    getHandDrawNames: function () {
                        var e = V(c, !0),
                            t = {
                                fileNames: e,
                                noSpace: !1,
                                count: e.length
                            };
                        return t
                    },
                    clearDrawPointsHis: function () {
                        try {
                            s = 0;
                            var t = V("drawpointshistag_");
                            if (t) {
                                var n, h = createIterator(t);
                                try {
                                    for (h.s(); !(n = h.n()).done;) {
                                        var i = n.value;
                                        try {
                                            uni.removeStorageSync(i)
                                        } catch (e) {
                                            r("log", "delete fail:", JSON.stringify(e), " at utils/funCtrl.js:166")
                                        }
                                    }
                                } catch (c) {
                                    h.e(c)
                                } finally {
                                    h.f()
                                }
                                return uni.removeStorageSync("fileTag_drawpointshistag_"), !0
                            }
                            return !1
                        } catch (e) {
                            r("log", "clearDrawPointsHis fail:", JSON.stringify(e), " at utils/funCtrl.js:175")
                        }
                        return !1
                    },
                    saveHandDrawClassName: function (t) {
                        try {
                            return uni.setStorageSync("handDrawFileClassTag_", t), !0
                        } catch (e) {
                            r("log", "saveHandDrawClassName fail:", JSON.stringify(e), " at utils/funCtrl.js:339")
                        }
                        return !1
                    },
                    getHandDrawClassName: function () {
                        var t = [];
                        try {
                            t = uni.getStorageSync("handDrawFileClassTag_")
                        } catch (e) {
                            r("log", "getHandDrawClassName fail:", JSON.stringify(e), " at utils/funCtrl.js:349")
                        }
                        t || (t = []);
                        for (var n = t.length; n < o.length; n++) t.push(o[n]);
                        return t
                    }
                }
            }).call(this, n("enhancedConsoleLogger")["default"])
        },

        "arrayToArrayLikeHelper": function (e, t, r) {
            var n = r("arrayLikeToArrayHelper");
            e.exports = function (e) {
                if (Array.isArray(e)) return n(e)
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },

        "b893": function (e, t) {
            e.exports = function (e) {
                if ("undefined" !== typeof Symbol && null != e[Symbol.iterator] || null != e["@@iterator"]) return Array.from(e)
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },

        "toConsumableArrayHelper": function (e, t, r) {
            var n = r("arrayLikeToArrayHelper");
            e.exports = function (e, t) {
                if (e) {
                    if ("string" === typeof e) return n(e, t);
                    var r = Object.prototype.toString.call(e).slice(8, -1);
                    return "Object" === r && e.constructor && (r = e.constructor.name), "Map" === r || "Set" === r ? Array.from(e) : "Arguments" === r || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(r) ? n(e, t) : void 0
                }
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },

        "nonIterableSpreadErrorHelper": function (e, t) {
            e.exports = function () {
                throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.")
            }, e.exports.__esModule = !0, e.exports["default"] = e.exports
        },
    },
    [
        ["mainAppEntry", "app-config"]
    ]




]);