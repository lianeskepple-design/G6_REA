namespace Creative.Platform.Devices.Features.Effects;

public struct EffectParameterRangeConfig<T>
{
	public T Min { get; }

	public T Max { get; }

	public T Granularity { get; }

	public T Default { get; }

	public EffectParameterRangeConfig(T min, T max, T granularity, T defaultVal)
	{
		Min = min;
		Max = max;
		Granularity = granularity;
		Default = defaultVal;
	}
}
