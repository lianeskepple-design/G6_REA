// Excerpt: Creative.Platform.Mixer.dll (2.4.67.0)
// Source: C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.Platform.Mixer.dll
// Decompiled with ilspycmd 9.1.0.7988 (ICSharpCode.Decompiler 9.1.0.7988)
// Type: CTSoundCore._stParamInfo

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace CTSoundCore;

[StructLayout(LayoutKind.Sequential, Pack = 4)]
[CompilerGenerated]
[TypeIdentifier("6712A0A7-C487-4A22-BB44-8A57D03265D8", "CTSoundCore._stParamInfo")]
public struct _stParamInfo
{
	public _stParam paramId;

	public _etParamType paramType;

	public uint paramDataSize;

	public _stParamValue minVal;

	public _stParamValue maxVal;

	public _stParamValue stepSize;

	public _stParamValue defaultVal;

	public uint paramAttrib;

	[MarshalAs(UnmanagedType.ByValArray, SizeConst = 32)]
	public sbyte[] szDescription;
}

// Type: Creative.Platform.Mixer.SoundCoreObject - GetParamInfoRange (excerpt)
	public bool GetParamInfoRange(MalcolmControlID AID, out double ADefValue, out double AMinValue, out double AMaxValue, out double AStepSize, out uint AParamAttribute)
	{
		ADefValue = 0.0;
		AMinValue = 0.0;
		AMaxValue = 0.0;
		AStepSize = 0.0;
		AParamAttribute = 0u;
		if (!bIsInitialized)
		{
			return false;
		}
		_stParam aParamID = NewParamID(AID);
		_stParamInfo AParamInfo = default(_stParamInfo);
		if (GetParamInfo(aParamID, ref AParamInfo))
		{
			if (AParamInfo.paramType == _etParamType.eParamType_Float)
			{
				ADefValue = AParamInfo.defaultVal.paramVal.floatVal;
				AMinValue = AParamInfo.minVal.paramVal.floatVal;
				AMaxValue = AParamInfo.maxVal.paramVal.floatVal;
				AStepSize = AParamInfo.stepSize.paramVal.floatVal;
				AParamAttribute = AParamInfo.paramAttrib;
				Util.PrintDebug("Get Param info " + AID.ToString() + ": Default " + AParamInfo.defaultVal.paramVal.floatVal + " / Min " + AParamInfo.minVal.paramVal.floatVal + " / Max " + AParamInfo.maxVal.paramVal.floatVal + " / Step " + AParamInfo.stepSize.paramVal.floatVal + " / Attribute " + AParamInfo.paramAttrib);
				return true;
			}
			if (AParamInfo.paramType == _etParamType.eParamType_Dword)
			{
				ADefValue = AParamInfo.defaultVal.paramVal.dwordVal;
				AMinValue = AParamInfo.minVal.paramVal.dwordVal;
				AMaxValue = AParamInfo.maxVal.paramVal.dwordVal;
				AStepSize = AParamInfo.stepSize.paramVal.dwordVal;
				AParamAttribute = AParamInfo.paramAttrib;
				Util.PrintDebug("Get Param info " + AID.ToString() + ": Default " + AParamInfo.defaultVal.paramVal.dwordVal + " / Min " + AParamInfo.minVal.paramVal.dwordVal + " / Max " + AParamInfo.maxVal.paramVal.dwordVal + " / Step " + AParamInfo.stepSize.paramVal.dwordVal + " / Attribute " + AParamInfo.paramAttrib);
				return true;
			}
			Util.PrintDebug("Type mis-match: " + AID);
			return false;
		}
		return false;
	}

	public bool GetParamBool(MalcolmControlID AID, out bool AEnable)
	{
		AEnable = false;
		_stParam aParamID = NewParamID(AID);
		_stParamValue AParamValue = default(_stParamValue);
		Util.PrintDebug("Start to get Param Value as bool: " + AID);
		if (GetParamValue(aParamID, ref AParamValue))
		{
			if (AParamValue.paramType == _etParamType.eParamType_Bool)
			{
				AEnable = AParamValue.paramVal.boolVal == 1;
				Util.PrintDebug("Get Param Bool " + AID.ToString() + " " + AEnable);
				return true;
			}
			Util.PrintDebug("Type mis-match: " + AID);
			return false;
		}
		Util.PrintDebug("Fail to get Param Value as bool: " + AID);
		return false;
	}

	public bool GetParamInt(MalcolmControlID AID, out uint AValue)
}

// Type: CTSoundCore.ISoundCore (interface, excerpt: param query slots)
	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void SetContext([In] _etContext contextId, [In] _etRestoreState restoreState);

	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void EnumFeatures([In] _etContext contextId, [In] uint index, out _stFeatureInfo featureInfo);

	void _VtblGap1_1();

	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void EnumParams([In] _etContext contextId, [In] uint index, [In] _etFeature featureId, out _stParamInfo paramInfo);

	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void GetParamInfo([In] _stParam param, out _stParamInfo paramInfo);

	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void GetParamValue([In] _stParam param, out _stParamValue paramValue);

	[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]
	void SetParamValue([In] _stParam param, [In] _stParamValue paramValue);

	void _VtblGap2_4();
