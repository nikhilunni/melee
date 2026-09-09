s32 lb_8000D148(f32 point0_x, f32 point0_y, f32 point1_x, f32 point1_y,
                f32 point2_x, f32 point2_y, f32 threshold)
{
    f32 dist_01;
    f32 var_f0;
    {
        f32 diff_01_x = point1_x - point0_x;
        f32 diff_01_y = point0_y - point1_y;
        f32 cross = point0_y * point1_x;

        dist_01 = (diff_01_x * diff_01_x) + (diff_01_y * diff_01_y);
        cross = (point0_x * point1_y) - cross;
        if (dist_01 < 0.00001f) {
            return 0;
        }
        dist_01 = sqrtf(dist_01);

        var_f0 = cross + ((diff_01_x * point2_x) + (diff_01_y * point2_y));
        if (var_f0 < 0.0f) {
            var_f0 = -var_f0;
        }
    }

    if ((var_f0 / dist_01) <= threshold) {
        f32 threshold_squared;
        f32 dist_squared_02;
        f32 dist_squared_12;

        dist_squared_02 = (point0_x - point2_x) * (point0_x - point2_x) +
                          (point0_y - point2_y) * (point0_y - point2_y);
        threshold_squared = threshold * threshold;
        dist_squared_12 = (point1_x - point2_x) * (point1_x - point2_x) +
                          (point1_y - point2_y) * (point1_y - point2_y);
        if (dist_squared_02 < threshold_squared) {
            if (dist_squared_12 > threshold_squared) {
                return 1;
            }
            if (dist_squared_12 < threshold_squared) {
                return 0;
            }
            return 1;
        }
        if (dist_squared_02 > threshold_squared) {
            if (dist_squared_12 > threshold_squared) {
                // If an axis of point0 and point1 are on opposite sides of
                // point2, return true.
                if (((point0_x > point2_x) && (point1_x < point2_x)) ||
                    ((point0_x < point2_x) && (point1_x > point2_x)) ||
                    ((point0_y > point2_y) && (point1_y < point2_y)) ||
                    ((point0_y < point2_y) && (point1_y > point2_y)))
                {
                    return 1;
                }
                return 0;
            }
            if (dist_squared_12 < threshold_squared) {
                return 1;
            }
            return 1;
        }
        return 1;
    }
    return 0;
}
