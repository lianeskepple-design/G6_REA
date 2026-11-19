using Creative.Platform.Devices.Features.Effects;
using Creative.Platform.Devices.Features.PropKeys;

namespace Creative.Platform.Devices.Features.Enrichers;

public class GraphicsEqFeatureEnricher : AbstractFeatureEnricher
{
	public override IFeature EnrichUndetermined(FeatureEnrichTarget target, string featureId)
	{
		IDeviceRepository deviceRepository = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.PropertyStore);
		IDeviceRepository deviceRepository2 = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.CDC);
		IDeviceRepository deviceRepository3 = target.DeviceRepo;
		if (deviceRepository != null && deviceRepository2 != null)
		{
			deviceRepository3 = deviceRepository2;
		}
		GraphicsEqFeature graphicsEqFeature = new GraphicsEqFeature(target.DeviceEndpoint, deviceRepository3, "GraphicEqFeatureId", deviceRepository3.GetKey("GraphicEqFeatureId"), isEnabledDefault: false);
		BasicIndexedEffectParameter effectParameter = new BasicIndexedEffectParameter(effectParameterRangeConfig: new EffectParameterRangeConfig<float>(-24f, 24f, 1f, 0f), deviceEndpoint: target.DeviceEndpoint, deviceRepository: deviceRepository3, scalableType: typeof(float), effectParameterId: "GraphicEqBandLevelEffectParameterId", firstIndexPropertykey: deviceRepository3.GetKey("GraphicEqBandLevelEffectParameterId"), numIndexes: CTTHXCommonPropKeys.CTPKEY_GraphicEQ_Gain9.pid + 1);
		graphicsEqFeature.AddEffectParameter(effectParameter);
		BasicNonIndexedEffectParameter effectParameter2 = new BasicNonIndexedEffectParameter(target.DeviceEndpoint, deviceRepository3, typeof(float), "GraphicPreAmpEffectParameterId", deviceRepository3.GetKey("GraphicPreAmpEffectParameterId"), new EffectParameterRangeConfig<float>(-12f, 12f, 1f, 0f));
		graphicsEqFeature.AddEffectParameter(effectParameter2);
		return graphicsEqFeature;
	}
}
