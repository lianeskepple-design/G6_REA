# 静态调用链与IL证据

## 7. 调用链（摘要；完整版见 `offline-contracts/call-chains.md`）

### 7.1 通路 A：SoundCore 硬件（AE 系列、G6 走 USB）

```
WPF ViewModel（EqBandViewModel / AggregatedFeatureViewModel）
  → AggregatedFeature / Feature（Devices.Features.GraphicsEqFeature / XBassXFi23Feature…）
  → BasicNonIndexedEffectParameter.SetValue<T>()        ← 边界检查只查 min/max
  → IDeviceRepository.SetValue → SoundCoreRepository.SetValue<T>
  → CTSoundCore.ISoundCore.SetParamValue(_stParam, _stParamValue)   （COM）
  → KsMalcCtl COM 服务（SndCrUSB.DLL）→ 驱动 → 硬件
```

### 7.2 通路 B：CDC（CTCDC / Malcolm 效果值）

```
… → CDCDeviceRepository.SetValue<T> → CDCRepositoryKey<G,S>.SetValue
  → CTCDCLibrary.CTCDCCMD.SetMalcolmEffectValue(CTCDCCMDPARAM_SetMalcolmEffectValue)
      paramType = CTCDCMALCOLMEFFECT_ParamType_GraphicEQBand0Gain (+ index)
      srcID     = CTCDCMALCOLMEFFECT_SourceID_NotApplicable
      flValue   = v        ← 原值（dB）
```

### 7.3 通路 C：APO（Windows 属性存储，`PropstoreEp = 0 / PropstoreFx = 1`）

```
… → PropStoreRepository.SetValue → PROPVARIANT(vt=4, AsFloat=v)
  → Creative.Platform.PropStore.PolicyConfigPropertyStore.SetPropertyValue(endpointId, type, PROPERTYKEY, pv)
  → 音频引擎属性存储 → APO（EFX 节点）
```

### 7.4 通路 D：运行时范围查询（未被 SBX 面板使用）

```
MalcolmControl.GetParamInfoRange(MalcolmControlID)
  → SoundCoreObject.GetParamInfoRange → NewParamID → GetParamInfo(_stParam) → _stParamInfo
  → scCtrl.GetParamInfo(param, out paramInfo)  ← CTSoundCore.ISoundCore，实现方 KsMalcCtl
```

---


## IL定位

关键指令及offset见 il-offsets.txt；定点源码见snippets/。运行时GetParamInfo值仍未知。没有声卡时不调用厂商COM服务。
