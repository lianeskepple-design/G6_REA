using Creative.Platform.Devices.Features.Effects;
using Creative.Platform.Devices.Features.SoundCore;
using Creative.Platform.Devices.Features.SpeakerConfigs;

namespace Creative.Platform.Devices.Features.Enrichers;

public class XBassXFi23FeatureEnricher : AbstractFeatureEnricher
{
	public override IFeature EnrichUndetermined(FeatureEnrichTarget target, string featureId)
	{
		IDeviceRepository deviceRepository = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.CDC);
		IDeviceRepository deviceRepository2 = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.PropertyStore);
		if (deviceRepository != null && deviceRepository2 != null)
		{
			BasicToggleableFeature basicToggleableFeature = new BasicToggleableFeature(target.DeviceEndpoint, target.DeviceRepo, "BassMgmtXBassEnableAttributeId", target.DeviceRepo.GetKey("BassMgmtXBassEnableAttributeId"), isEnabledDefault: false);
			basicToggleableFeature.AddEffectParameter(new BasicNonIndexedEffectParameter(target.DeviceEndpoint, target.DeviceRepo, typeof(float), "XBassCrossOverFrequencyEffectParameterId", target.DeviceRepo.GetKey("XBassCrossOverFrequencyEffectParameterId"), new EffectParameterRangeConfig<float>(10f, 1000f, 1f, 80f)));
			basicToggleableFeature.InterestedKeys.Add(target.DeviceRepo.GetKey("XBassCrossOverFrequencyEffectParameterId"));
			basicToggleableFeature.AddEffectParameter(new BasicNonIndexedEffectParameter(effectParameterRangeConfig: new EffectParameterRangeConfig<float>(0f, 100f, 1f, 50f), deviceEndpoint: target.DeviceEndpoint, deviceRepository: target.DeviceRepo, scalableType: typeof(float), effectParameterId: "XBassStrengthEffectParameterId", currentValuePropertykey: target.DeviceRepo.GetKey("XBassStrengthEffectParameterId")));
			return basicToggleableFeature;
		}
		XBassXFi23Feature xBassXFi23Feature = new XBassXFi23Feature(target.DeviceEndpoint, target.DeviceRepo, "XBassXFi23FeatureId", target.DeviceRepo.GetKey("XBassXFi23FeatureId"), isEnabledDefault: false);
		xBassXFi23Feature.AddEffectParameter(new BasicNonIndexedEffectParameter(target.DeviceEndpoint, target.DeviceRepo, typeof(float), "XBassCrossOverFrequencyEffectParameterId", target.DeviceRepo.GetKey("XBassCrossOverFrequencyEffectParameterId"), new EffectParameterRangeConfig<float>(10f, 1000f, 1f, 80f)));
		xBassXFi23Feature.InterestedKeys.Add(target.DeviceRepo.GetKey("XBassCrossOverFrequencyEffectParameterId"));
		xBassXFi23Feature.HasBassManagement = HasBassManagement(target);
		if (xBassXFi23Feature.HasBassManagement)
		{
			xBassXFi23Feature.AddEffectParameter(new BasicNonIndexedEffectParameter(target.DeviceEndpoint, target.DeviceRepo, typeof(float), "BassMgmtCrossOverFreqEffectParameterId", target.DeviceRepo.GetKey("BassMgmtCrossOverFreqEffectParameterId"), new EffectParameterRangeConfig<float>(10f, 1000f, 1f, 80f)));
			xBassXFi23Feature.InterestedKeys.Add(target.DeviceRepo.GetKey("BassMgmtCrossOverFreqEffectParameterId"));
			xBassXFi23Feature.AddEffectParameter(new ByValueEffectParameter<bool>(target.DeviceEndpoint, target.DeviceRepo, "BassMgmtSubwooferGainId", target.DeviceRepo.GetKey("BassMgmtSubwooferGainId")));
			xBassXFi23Feature.InterestedKeys.Add(target.DeviceRepo.GetKey("BassMgmtSubwooferGainId"));
		}
		EffectParameterRangeConfig<float> effectParameterRangeConfig = new EffectParameterRangeConfig<float>(0f, 1f, 0.01f, 0.5f);
		EffectParameterRangeConfig<float> effectParameterRangeConfig2 = new EffectParameterRangeConfig<float>(0f, 100f, 1f, 50f);
		EffectParameterRangeConfig<float> effectParameterRangeConfig3 = effectParameterRangeConfig2;
		if (target.DeviceRepo is SoundCoreRepository && target.DeviceEndpoint.ProductCodeName != "Kratos2")
		{
			effectParameterRangeConfig3 = effectParameterRangeConfig;
		}
		xBassXFi23Feature.AddEffectParameter(new BasicNonIndexedEffectParameter(target.DeviceEndpoint, target.DeviceRepo, typeof(float), "XBassStrengthEffectParameterId", target.DeviceRepo.GetKey("XBassStrengthEffectParameterId"), effectParameterRangeConfig3));
		xBassXFi23Feature.SpeakerConfigFeature = FeatureEnricherPool.Instance.Enrich(target, "SpeakerConfigFeatureId") as ISpeakerConfigFeature;
		xBassXFi23Feature.XBassTogglePropertykey = target.DeviceRepo.GetKey("BassMgmtXBassEnableAttributeId");
		return xBassXFi23Feature;
	}

	private bool HasBassManagement(FeatureEnrichTarget target)
	{
		return target.DeviceEndpoint.ProductCodeName != "G6";
	}
}
