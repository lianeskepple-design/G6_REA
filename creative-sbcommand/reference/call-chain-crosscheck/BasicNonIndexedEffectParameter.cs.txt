using System;
using Creative.Platform.Devices.Models;

namespace Creative.Platform.Devices.Features.Effects;

public class BasicNonIndexedEffectParameter : BasicEffectParameter, INonIndexedEffectParameter, IEffectParameter, IDevicePropertySupported, IRestoreable, IDevicePropertyNotifiable
{
	public IDeviceRepositoryKey CurrentValuePropertykey { get; set; }

	public object EffectParameterRangeConfig { get; }

	public BasicNonIndexedEffectParameter(DeviceEndpoint deviceEndpoint, IDeviceRepository deviceRepository, Type scalableType, string effectParameterId, IDeviceRepositoryKey currentValuePropertykey, object effectParameterRangeConfig)
		: base(deviceEndpoint, deviceRepository, scalableType, effectParameterId)
	{
		CurrentValuePropertykey = currentValuePropertykey;
		base.InterestedKeys.Add(CurrentValuePropertykey);
		EffectParameterRangeConfig = effectParameterRangeConfig;
	}

	public virtual EffectParameterRangeConfig<T> GetEffectScaleConfig<T>()
	{
		MatchConfiguredType(typeof(T));
		return (EffectParameterRangeConfig<T>)EffectParameterRangeConfig;
	}

	public virtual T GetValue<T>()
	{
		MatchConfiguredType(typeof(T));
		return base.DeviceRepository.GetValue<T>(CurrentValuePropertykey);
	}

	public virtual void SetValue<T>(T value)
	{
		MatchConfiguredType(typeof(T));
		if (!BasicEffectParameter.EffectParamBoundaryCheckFuncDict.ContainsKey(typeof(T)) || EffectParameterRangeConfig == null || BasicEffectParameter.EffectParamBoundaryCheckFuncDict[typeof(T)](value, EffectParameterRangeConfig))
		{
			base.DeviceRepository.SetValue(CurrentValuePropertykey, value);
		}
	}

	public override void Restore()
	{
		BasicEffectParameter.EffectParamRestoreActionDict[base.ScalableType](base.DeviceRepository, CurrentValuePropertykey, EffectParameterRangeConfig);
	}
}
