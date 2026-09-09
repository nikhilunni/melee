static void HSD_PadScale(HSD_PadStatus* mp)
{
    PadLibData* p = &HSD_PadLibData;

    mp->nml_stickX = (f32) mp->stickX / (f32) p->scale_stick;
    mp->nml_stickY = (f32) mp->stickY / (f32) p->scale_stick;
    mp->nml_subStickX = (f32) mp->subStickX / (f32) p->scale_stick;
    mp->nml_subStickY = (f32) mp->subStickY / (f32) p->scale_stick;
    mp->nml_analogL = (f32) mp->analogL / (f32) p->scale_analogLR;
    mp->nml_analogR = (f32) mp->analogR / (f32) p->scale_analogLR;
    mp->nml_analogA = (f32) mp->analogA / (f32) p->scale_analogAB;
    mp->nml_analogB = (f32) mp->analogB / (f32) p->scale_analogAB;
}
