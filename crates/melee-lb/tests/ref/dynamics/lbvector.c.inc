float lbVector_Normalize(Vec3* vec)
{
    float len = lbVector_Len(vec);
    float inv;

    if (len == 0.0f) {
        return 0.0f;
    }
    inv = 1.0f / len;
    vec->x *= inv;
    vec->y *= inv;
    vec->z *= inv;
    return len;
}

float lbVector_Angle(Vec3* a, Vec3* b)
{
    float lena_lenb = lbVector_Len(a) * lbVector_Len(b);

    if (lena_lenb > 0.0000000001f) {
        float cosine = (a->x * b->x + a->y * b->y + a->z * b->z) / lena_lenb;
        if (cosine > 1.0f) {
            cosine = 1.0f;
        }
        if (cosine < -1.0f) {
            cosine = -1.0f;
        }

        return acosf(cosine); // acos
    }
    return 0.0f;
}

static float lbvector_sin(float angle)
{
    if (angle > M_PI) {
        angle -= M_TAU;
    } else if (angle < -M_PI) {
        angle += M_TAU;
    }
    return 0.9878619909286499f * angle -
           0.15527099370956421f * angle * angle * angle +
           0.0056429998949170113f * angle * angle * angle * angle * angle;
}

static float lbvector_cos(float angle)
{
    angle += M_PI / 2;
    if (angle > M_PI) {
        angle -= M_TAU;
    } else if (angle < -M_PI) {
        angle += M_TAU;
    }
    return 0.9878619909286499f * angle -
           0.15527099370956421f * angle * angle * angle +
           0.0056429998949170113f * angle * angle * angle * angle * angle;
}

void lbVector_RotateAboutUnitAxis(Vec3* v, Vec3* axis, float angle)
{
    // The implementation is unnecessarily complex. the idea is to reduce the
    // problem to the case where the rotation axis is pointing along the z-axis
    // by using two initial rotations, then the new v is rotated about the
    // z-axis by angle, and finally the first two rotations are reversed.

    float len_axis_yz = sqrtf(axis->y * axis->y + axis->z * axis->z);
    float s = lbvector_sin(angle);
    float c = lbvector_cos(angle);
    float unit_axis_yz_y;
    float unit_axis_yz_z;
    float x, y, z;
    float x2, z2;
    float x3, y3;

    // rotation (1) about the x-axis: rotate everything such that the rotation
    // axis lies in the xz-plane. new v is then (x,y,z), new rotation axis is
    // (b.x, 0, len_axis_yz)
    if (len_axis_yz > 0.0000000001f) {
        unit_axis_yz_y = axis->z / len_axis_yz;
        unit_axis_yz_z = axis->y / len_axis_yz;

        x = v->x;
        y = v->y * unit_axis_yz_y - v->z * unit_axis_yz_z;
        z = v->y * unit_axis_yz_z + v->z * unit_axis_yz_y;
    } else {
        x = v->x;
        y = v->y;
        z = v->z;
    }

    // rotation (2) about the y-axis: rotate everything such that the rotation
    // axis aligns with the z-axis new v is then (x2,y2,z2)
    x2 = x * len_axis_yz - z * axis->x;
    // y2 = y
    z2 = x * axis->x + z * len_axis_yz;

    // rotate by 'angle' about the z axis, which now aligns with the rotation
    // axis
    x3 = x2 * c - y * s; // remember that y2=y
    y3 = x2 * s + y * c;
    // z3 = z2

    // opposite of rotation (2). We overwrite (x,y,z) with the resulting new v.
    x = x3 * len_axis_yz + z2 * axis->x; // remember that z3=z2
    y = y3;
    z = -x3 * axis->x + z2 * len_axis_yz;

    // opposite of rotation (1)
    if (len_axis_yz > 0.0000000001f) {
        v->x = x;
        v->y = y * unit_axis_yz_y + z * unit_axis_yz_z;
        v->z = -y * unit_axis_yz_z + z * unit_axis_yz_y;
    } else {
        v->x = x;
        v->y = y;
        v->z = z;
    }
}
