using Creative.Platform.Devices.Features.Effects;

namespace Creative.Platform.Devices.Features.Enrichers;

public class EaxEnvironmentFeatureEnricher : AbstractFeatureEnricher
{
	public override IFeature EnrichUndetermined(FeatureEnrichTarget target, string featureId)
	{
		BasicToggleableFeature basicToggleableFeature = new BasicToggleableFeature(target.DeviceEndpoint, target.DeviceRepo, "EAXEnvironmentFeatureId", target.DeviceRepo.GetKey("EAXEnvironmentFeatureId"), isEnabledDefault: false);
		basicToggleableFeature.AddEffectParameter(new BasicNonIndexedEffectParameter(target.DeviceEndpoint, target.DeviceRepo, typeof(long), "EaxEnvironmentEffectParameterId", target.DeviceRepo.GetKey("EaxEnvironmentEffectParameterId"), new EffectParameterRangeConfig<long>(-6000L, 1200L, 10L, 0L)));
		return basicToggleableFeature;
	}
}
