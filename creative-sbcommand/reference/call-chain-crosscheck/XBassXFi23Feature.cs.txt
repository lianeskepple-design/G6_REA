using System.Linq;
using System.Reflection;
using Creative.Platform.Devices.Features.Effects;
using Creative.Platform.Devices.Features.SpeakerConfigs;
using Creative.Platform.Devices.Models;
using log4net;

namespace Creative.Platform.Devices.Features;

public class XBassXFi23Feature : BasicToggleableFeature
{
	private new static readonly ILog Logger = LogManager.GetLogger(MethodBase.GetCurrentMethod().DeclaringType);

	private IDeviceRepositoryKey xBassTogglePropertykey;

	public ISpeakerConfigFeature SpeakerConfigFeature { get; set; }

	public bool HasBassManagement { get; set; }

	public IDeviceRepositoryKey XBassTogglePropertykey
	{
		get
		{
			return xBassTogglePropertykey;
		}
		set
		{
			base.InterestedKeys.Remove(xBassTogglePropertykey);
			xBassTogglePropertykey = value;
			base.InterestedKeys.Add(xBassTogglePropertykey);
		}
	}

	public IDeviceRepositoryKey BassManagementToggleKey { get; set; }

	public override IDeviceRepositoryKey GetEnablePropertykey()
	{
		if (!HasBassManagement)
		{
			return XBassTogglePropertykey;
		}
		SpeakerConfig speakerConfig = SpeakerConfigFeature?.GetSpeakerConfig();
		if (speakerConfig == null)
		{
			return XBassTogglePropertykey;
		}
		if (speakerConfig.SpeakerMode != SpeakerMode.SpeakerModeHeadPhone)
		{
			if (!SpeakerConfig.HasSubwoofer(speakerConfig))
			{
				return XBassTogglePropertykey;
			}
			return BassManagementToggleKey;
		}
		return XBassTogglePropertykey;
	}

	public override IEffectParameter GetEffectParameter(string effectParameterId)
	{
		if (HasBassManagement)
		{
			SpeakerConfig speakerConfig = SpeakerConfigFeature?.GetSpeakerConfig();
			if ("XBassCrossOverFrequencyEffectParameterId".Equals(effectParameterId) && speakerConfig != null && speakerConfig.SpeakerMode != SpeakerMode.SpeakerModeHeadPhone && SpeakerConfig.HasSubwoofer(speakerConfig))
			{
				string effectParamId = "BassMgmtCrossOverFreqEffectParameterId";
				return base.EffectParameters.FirstOrDefault((IEffectParameter effect) => effect.EffectParameterId.Equals(effectParamId));
			}
		}
		return base.GetEffectParameter(effectParameterId);
	}

	public XBassXFi23Feature(DeviceEndpoint deviceEndpoint, IDeviceRepository deviceRepository, string featureId, IDeviceRepositoryKey enableKey, bool isEnabledDefault)
		: base(deviceEndpoint, deviceRepository, featureId, enableKey, isEnabledDefault)
	{
		BassManagementToggleKey = enableKey;
		HasBassManagement = true;
	}

	public override T GetValue<T>(string effectId)
	{
		if ("BassMgmtSubwooferGainId".Equals(effectId))
		{
			IEffectParameter effectParameter = GetEffectParameter(effectId);
			if (effectParameter is ByValueEffectParameter<T>)
			{
				ByValueEffectParameter<T> byValueEffectParameter = GetEffectParameter(effectId) as ByValueEffectParameter<T>;
				return byValueEffectParameter.GetValue();
			}
		}
		return base.GetValue<T>(effectId);
	}

	public override void SetValue<T>(T newValue, string effectId)
	{
		if ("BassMgmtSubwooferGainId".Equals(effectId))
		{
			IEffectParameter effectParameter = GetEffectParameter(effectId);
			if (effectParameter is ByValueEffectParameter<T>)
			{
				ByValueEffectParameter<T> byValueEffectParameter = effectParameter as ByValueEffectParameter<T>;
				byValueEffectParameter.SetValue(newValue);
			}
		}
		else
		{
			base.SetValue(newValue, effectId);
		}
	}
}
