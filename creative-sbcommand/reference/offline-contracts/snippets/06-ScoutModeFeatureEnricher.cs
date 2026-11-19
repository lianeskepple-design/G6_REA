namespace Creative.Platform.Devices.Features.Enrichers;

public class ScoutModeFeatureEnricher : AbstractFeatureEnricher
{
	public override IFeature EnrichUndetermined(FeatureEnrichTarget target, string featureId)
	{
		IDeviceRepository deviceRepository = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.SoundCore);
		IDeviceRepository deviceRepository2 = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.PropertyStore);
		IDeviceRepository deviceRepository3 = target.DeviceEndpoint.GetDeviceRepository(DeviceRepositoryType.CDC);
		if (deviceRepository != null)
		{
			SoundCoreScoutFeature soundCoreScoutFeature = new SoundCoreScoutFeature(target.DeviceEndpoint, target.DeviceRepo, "ScoutFeatureId", target.DeviceRepo.GetKey("ScoutFeatureId"), isEnabledDefault: false);
			soundCoreScoutFeature.Surround = target.DeviceEndpoint.GetFeature("CMSSSurroundFeatureId") as BasicToggleableFeature;
			soundCoreScoutFeature.Crystalizer = target.DeviceEndpoint.GetFeature("CrystalizerFeatureId") as BasicToggleableFeature;
			soundCoreScoutFeature.XBass = target.DeviceEndpoint.GetFeature("XBassXFi23FeatureId") as BasicToggleableFeature;
			soundCoreScoutFeature.SVM = target.DeviceEndpoint.GetFeature("SVMFeatureId") as BasicToggleableFeature;
			soundCoreScoutFeature.DialogPlus = target.DeviceEndpoint.GetFeature("DialogAdvantageFeatureId") as BasicToggleableFeature;
			soundCoreScoutFeature.GraphicsEQ = target.DeviceEndpoint.GetFeature("GraphicEqFeatureId") as BasicToggleableFeature;
			return soundCoreScoutFeature;
		}
		if (deviceRepository2 != null)
		{
			if (deviceRepository3 != null)
			{
				return new BasicToggleableFeature(target.DeviceEndpoint, target.DeviceRepo, "ScoutFeatureId", target.DeviceRepo.GetKey("ScoutFeatureId"), isEnabledDefault: false);
			}
			BasicToggleableFeature basicToggleableFeature = FeatureEnricherPool.Instance.Enrich(target, "MasterOnOffFeatureId") as BasicToggleableFeature;
			BasicToggleableFeature basicToggleableFeature2 = FeatureEnricherPool.Instance.Enrich(target, "GraphicEqFeatureId") as BasicToggleableFeature;
			ApoScoutFeature apoScoutFeature = new ApoScoutFeature(target.DeviceEndpoint, target.DeviceRepo, "ScoutFeatureId", target.DeviceRepo.GetKey("ScoutFeatureId"), isEnabledDefault: false)
			{
				MasterOnOff = basicToggleableFeature,
				GraphicEqualizer = basicToggleableFeature2
			};
			if (basicToggleableFeature != null)
			{
				apoScoutFeature.InterestedKeys.Add(basicToggleableFeature.EnableKey);
			}
			if (basicToggleableFeature2 != null)
			{
				apoScoutFeature.InterestedKeys.Add(basicToggleableFeature2.EnableKey);
			}
			apoScoutFeature.InterestedKeys.Add(target.DeviceRepo.GetKey("Reality3DFeatureId"));
			if (FeatureEnricherPool.Instance.Enrich(target, "EAXEnvironmentFeatureId") is BasicToggleableFeature basicToggleableFeature3)
			{
				apoScoutFeature.EAXEnvironment = basicToggleableFeature3;
				apoScoutFeature.InterestedKeys.Add(basicToggleableFeature3.EnableKey);
			}
			return apoScoutFeature;
		}
		return null;
	}
}
