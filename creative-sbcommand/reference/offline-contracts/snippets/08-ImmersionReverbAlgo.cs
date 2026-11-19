using Creative.Platform.Devices.Features.Effects;

namespace Creative.Platform.Devices.Features;

public class ImmersionReverbAlgo
{
	public bool ShouldSetReverb(EffectParameterRangeConfig<float> surroundRange, float surroundLevel, float startReverbThresholdPct)
	{
		float num = surroundRange.Max - surroundRange.Min;
		float num2 = startReverbThresholdPct * num;
		return surroundLevel > num2;
	}

	public long GetReverbValue(EffectParameterRangeConfig<float> surroundRange, float surroundLevel, EffectParameterRangeConfig<long> eaxReverbRange, float startReverbThresholdPct, float reverbMaxPct)
	{
		float num = surroundRange.Max - surroundRange.Min;
		float num2 = startReverbThresholdPct * num;
		float num3 = surroundRange.Max - num2;
		float num4 = (surroundLevel - num2) / num3;
		long num5 = (int)((float)(eaxReverbRange.Max - eaxReverbRange.Min) * reverbMaxPct);
		return (long)(num4 * (float)num5 + (float)eaxReverbRange.Min);
	}
}
