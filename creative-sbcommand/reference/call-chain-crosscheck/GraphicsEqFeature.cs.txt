using Creative.Platform.Devices.Models;

namespace Creative.Platform.Devices.Features;

public class GraphicsEqFeature : BasicToggleableFeature
{
	private static readonly int[] EqBandFrequecies = new int[10] { 31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000 };

	public GraphicsEqFeature(DeviceEndpoint deviceEndpoint, IDeviceRepository deviceRepository, string featureId, IDeviceRepositoryKey enableKey, bool isEnabledDefault)
		: base(deviceEndpoint, deviceRepository, featureId, enableKey, isEnabledDefault)
	{
	}

	public virtual int[] GetEqBandFrequencies()
	{
		return EqBandFrequecies;
	}

	public virtual int GetEqBandCount()
	{
		return EqBandFrequecies.Length;
	}

	public virtual int GetEqBandFrequency(int index)
	{
		return EqBandFrequecies[index];
	}
}
