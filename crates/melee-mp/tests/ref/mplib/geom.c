static void mpRemap2d(float* x_out, float* y_out, float ax0, float ay0,
                      float ax1, float ay1, float bx0, float by0, float bx1,
                      float by1, float px, float py)
{
    double dx;
    double dy;
    double dist2;
    float f30;
    float f29;
    dx = ax1 - ax0;
    dy = ay1 - ay0;
    f30 = px - ax0;
    f29 = py - ay0;
    dist2 = (dy * dy) + (dx * dx);
    if (ABS(dist2) > 0.0001) {
        // how far along line a is point p
        double t = (dy * f29 + dx * f30) / dist2;
        if (t > 1.0) {
            t = 1.0;
        } else if (t < 0.0) {
            t = 0.0;
        }

        *x_out = px + (1.0 - t) * (bx0 - ax0) + t * (bx1 - ax1);
        *y_out = py + (1.0 - t) * (by0 - ay0) + t * (by1 - ay1);
    } else {
        *x_out = px + (bx0 - ax0) + (bx1 - ax0);
        *y_out = py + (by0 - ay0) + (by1 - ay0);
    }
}

bool mpLineIntersection(float a0x, float a0y, float a1x, float a1y, float b0x,
                        float b0y, float b1x, float b1y, float* int_x,
                        float* int_y)
{
    bool b1_below_a = false;
    bool b2_above_a = false;

    // b entirely left/right of a
    if (a0x <= a1x) {
        if ((b0x < a0x && b1x < a0x) || (a1x < b0x && a1x < b1x)) {
            return false;
        }
    } else {
        if ((b0x < a1x && b1x < a1x) || (a0x < b0x && a0x < b1x)) {
            return false;
        }
    }

    // b entirely above/below a
    if (a0y <= a1y) {
        if ((b0y < a0y && b1y < a0y) || (a1y < b0y && a1y < b1y)) {
            return false;
        }
    } else {
        if ((b0y < a1y && b1y < a1y) || (a0y < b0y && a0y < b1y)) {
            return false;
        }
    }

    {
        double ah = a1y - a0y;
        double d0x = b0x - a0x;
        double aw = a1x - a0x;
        double d0y = b0y - a0y;
        double hs_b0_a = (aw * d0y) - (ah * d0x);
        double d1y;
        double d1x;
        double det;
        double hs_b1_a;
        double bh;
        double bw;

        if (hs_b0_a < 0.0) {
            if (hs_b0_a < -0.1) {
                return false;
            }
            b1_below_a = true;
        }

        d1x = b1x - a1x;
        d1y = b1y - a1y;

        hs_b1_a = (aw * d1y) - (ah * d1x);
        if (hs_b1_a > 0.0) {
            if (hs_b1_a > 0.1) {
                return false;
            }
            b2_above_a = true;
        }

        // check if a and b are colinear
        if (hs_b0_a == 0.0 && hs_b1_a == 0.0) {
            return false;
        }

        det = (d0x * d1y) - (d0y * d1x);
        if (det < hs_b0_a) {
            if (det < hs_b1_a) {
                return false;
            }
        } else if (det > hs_b0_a) {
            if (det > hs_b1_a) {
                return false;
            }
        }

        bw = b1x - b0x;
        bh = b1y - b0y;
        if (!((bw == 0.0 && bh == 0.0) || (b1_below_a && b2_above_a) ||
              (hs_b0_a >= 0.0 && b2_above_a)))
        {
            double area = (bw * ah) - (bh * aw);

            if (ABS(area) > 0.0001F) {
                double t =
                    ((bw * d0y) - (bh * d0x)) / area; // barycentric weight
                if (t > 0.0) {
                    if (t < 1.0) {
                        *int_x = (aw * t) + a0x;
                        *int_y = (ah * t) + a0y;
                    } else {
                        *int_x = a1x;
                        *int_y = a1y;
                    }
                } else {
                    *int_x = a0x;
                    *int_y = a0y;
                }

                goto tlabel;
            }
        }
        return false;
    tlabel:
        return true;
    }
}

bool mpLineIntersectionH(float* int_x, float* int_y, float a0x, float a0y,
                         float a1x, float b0x, float b0y, float b1x, float b1y)
{
    float max_ax;
    float min_ax;
    double dbx;
    double dby;
    double new_x;
    double dx;

    if (a0x < a1x) {
        if ((b0x < a0x && b1x < a0x) || (a1x < b0x && a1x < b1x)) {
            return false;
        }
        if (b0y - a0y < -0.0001 || b1y - a0y > 0.0001) {
            return false;
        }
        min_ax = a0x;
        max_ax = a1x;
    } else {
        if ((b0x < a1x && b1x < a1x) || (a0x < b0x && a0x < b1x)) {
            return false;
        }
        if (b1y - a0y < -0.0001 || b0y - a0y > 0.0001) {
            return false;
        }
        min_ax = a1x;
        max_ax = a0x;
    }
    dby = b1y - b0y;
    dbx = b1x - b0x;
    if (ABS(dby) < 0.0001) {
        return false;
    }
    new_x = dbx / dby * (a0y - b0y) + b0x;
    dx = new_x - min_ax;
    if (dx < 0.0) {
        if (dx < -0.1) {
            return false;
        }
        new_x = min_ax;
    }
    if (new_x - max_ax > 0.0) {
        if (new_x - max_ax > 0.1) {
            return false;
        }
        new_x = max_ax;
    }
    *int_x = new_x;
    *int_y = a0y;
    return true;
}

bool mpLineIntersectionV(float* int_x, float* int_y, float a0x, float a0y,
                         float a1y, float b0x, float b0y, float b1x, float b1y)
{
    float min_ay;
    float max_ay;
    double dbx;
    double dby;
    double new_y;
    double dy;

    if (a0y < a1y) {
        if ((b0y < a0y && b1y < a0y) || (a1y < b0y && a1y < b1y)) {
            return false;
        }
        if (b1x - a0x < -0.0001 || b0x - a0x > 0.0001) {
            return false;
        }
        min_ay = a0y;
        max_ay = a1y;
    } else {
        if ((b0y < a1y && b1y < a1y) || (a0y < b0y && a0y < b1y)) {
            return false;
        }
        if (b0x - a0x < -0.0001 || b1x - a0x > 0.0001) {
            return false;
        }
        min_ay = a1y;
        max_ay = a0y;
    }
    dby = b1y - b0y;
    dbx = b1x - b0x;
    if (ABS(dbx) < 0.0001) {
        return false;
    }
    new_y = (dby / dbx * (a0x - b0x)) + b0y;
    dy = new_y - min_ay;
    if (dy < 0.0) {
        if (dy < -0.1) {
            return false;
        }
        new_y = min_ay;
    }
    dy = new_y - max_ay;
    if (dy > 0.0) {
        if (dy > 0.1) {
            return false;
        }
        new_y = max_ay;
    }
    *int_x = a0x;
    *int_y = new_y;
    return true;
}
