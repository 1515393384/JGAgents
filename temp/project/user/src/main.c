/*********************************************************************************************************************
 * CH32V307VCT6 Opensourec Library 即（CH32V307VCT6 开源库）是一个基于官方 SDK 接口的第三方开源库
 * Copyright (c) 2022 SEEKFREE 逐飞科技
 *
 * 本文件是CH32V307VCT6 开源库的一部分
 *
 * CH32V307VCT6 开源库 是免费软件
 * 您可以根据自由软件基金会发布的 GPL（GNU General Public License，即 GNU通用公共许可证）的条款
 * 即 GPL 的第3版（即 GPL3.0）或（您选择的）任何后来的版本，重新发布和/或修改它
 *
 * 本开源库的发布是希望它能发挥作用，但并未对其作任何的保证
 * 甚至没有隐含的适销性或适合特定用途的保证
 * 更多细节请参见 GPL
 *
 * 您应该在收到本开源库的同时收到一份 GPL 的副本
 * 如果没有，请参阅<https://www.gnu.org/licenses/>
 *
 * 额外注明：
 * 本开源库使用 GPL3.0 开源许可证协议 以上许可申明为译文版本
 * 许可申明英文版在 libraries/doc 文件夹下的 GPL3_permission_statement.txt 文件中
 * 许可证副本在 libraries 文件夹下 即该文件夹下的 LICENSE 文件
 * 欢迎各位使用并传播本程序 但修改内容时必须保留逐飞科技的版权声明（即本声明）
 *
 * 文件名称          main
 * 公司名称          成都逐飞科技有限公司
 * 版本信息          查看 libraries/doc 文件夹内 version 文件 版本说明
 * 开发环境          MounRiver Studio V1.8.1
 * 适用平台          CH32V307VCT6
 * 店铺链接          https://seekfree.taobao.com/
 *
 * 修改记录
 * 日期                                      作者                             备注
 * 2022-09-15        大W            first version
 ********************************************************************************************************************/

/*
 * 场地实测结构（重要，决定判据）：
 *   整场铺蓝色垫子；赛道是贴在蓝垫上的白色宽带，宽 45.5cm。
 *   白带两侧【没有黑线】，白 -> 蓝 直接过渡。
 *   蓝区里的黑线是垫子接缝；白带表面本身也有横向拼缝、胶带、脏痕。
 *   => 灰度上：白带最亮，蓝垫中等偏暗，接缝/胶带很暗。
 *   => 所以“暗”既可能是赛道外，也可能是白带上的脏点，必须区分。
 *
 * 主循环 main step（while 1 内顺序执行）：
 * step 1 检测入口：等摄像头新帧；100ms 无帧按丢线停车
 * step 2 二值化：近场 ROI 大津阈值 + 近场白占比校验
 * step 3 找线：近场基准行实测道宽 -> 透视缩放 -> 逐行带连续性约束跟踪
 * step 4 闭环控制：按实测半宽归一化 PD + 弯道自减速 + 丢线保持
 * step 5 状态显示：每 10 帧刷新 TFT 遥测
 *
 * 本轮相对上一版的关键修正（对应实测“抽搐 / 一直 LOST 闪烁 / 反打 / 直线不走 / 弯道更剧烈”）：
 * 1 边缘判定从“连续2个暗像素”提高到“连续3个暗像素”，白带上的拼缝/脏点/胶带不再被当成赛道边
 * 2 新增逐行连续性约束：本行 l/r 与上一行相差超过 CONTINUITY_PX 就丢弃该行，不污染中心
 * 3 赛道像素宽度改为“近场基准行实测”驱动，不再用写死的 62/32 先验 —— 装高、45.5cm 都能自适应
 * 4 转向误差用实测半宽做分母，误差半个赛道宽才顶满舵
 * 5 权重改为近场优先（车头正前方才是当前姿态），弯道不再提前打死
 * 6 base_speed 1350 -> 3000：13.5% 占空比带不动这车重，直线“走两步停一下”
 */
#include "zf_common_headfile.h"

#define servo TIM2_PWM_MAP1_CH1_A15
#define motor1_in1 TIM4_PWM_MAP1_CH1_D12
#define motor1_in2 TIM4_PWM_MAP1_CH2_D13
#define motor2_in1 TIM1_PWM_MAP3_CH2_E11
#define motor2_in2 TIM1_PWM_MAP3_CH1_E9

#define servo_center        750     // 舵机机械中值（偏左/偏右时微调）
#define SERVO_DIR           1       // 舵机方向：若车偏左反而往左打，改成 -1
#define MAX_SERVO_TURN      150     // 舵机最大打角限幅（防顶死悬架）

#define KP_NORM             1.00f   // 归一化转向比例（1.0 = 偏差半个赛道宽刚好打满舵）
#define KD_NORM             0.22f   // 归一化转向阻尼
#define D_TERM_LIMIT        0.30f   // 微分项最多占总打角的比例（防恢复帧冲击）
#define ERROR_LPF_ALPHA     0.60f   // 误差低通滤波系数
#define DEADBAND_PX         2.0f    // 中心小死区（像素），死区内线性收敛不留阶跃

#define base_speed          3000    // 直道基础占空比（满分 10000）。13.5% 带不动车，起步用 30%
#define turn_min_speed      2100    // 急弯最低保底占空比（防弯道降速堵转）
#define max_speed           7000    // 最大限速保护
#define TURN_SLOW_RATIO     0.30f   // 满舵时最多降速 30%
#define SPEED_RAMP_STEP     200     // 电机占空比每帧最大变化量（消除停-走冲击）

// ── 图像识别参数（适配 45.5cm 白色赛道 + 蓝色垫子背景）──────────────────────────
#define MIN_CONTRAST        12      // 近场前后景最小灰度差
#define NEAR_WHITE_MIN_PCT  15      // 近场白占比下限（%）。45.5cm 白带近场约占 40~60%
#define NEAR_SEG_MIN_W      30      // 近场基准行最少要找到多宽的白段才算“看到赛道”
#define CROSS_ROWS_RATIO    0.55f   // 两侧全白到边界的行占比超过该值判为十字
#define CENTER_OK_RATIO     0.10f   // 居中显示阈值（占半宽比例）
#define FRAME_TIMEOUT_MS    100
#define TFT_REFRESH_DIV     10      // 每 10 帧刷一次屏幕
#define LOST_HOLD_FRAMES    8       // 连续丢线容忍帧数，之内保持舵机滑行
#define LOST_CREEP_PCT      55      // 丢线滑行占基础速度的百分比

#define SCAN_BOT_ROW        (MT9V03X_H - 8)
#define SCAN_TOP_ROW        44
#define SCAN_STEP           3
#define SCAN_ROWS           (((SCAN_BOT_ROW - SCAN_TOP_ROW) / SCAN_STEP) + 1)

// 近场 ROI：视野下部，赛道一定在这里；远端背景不参与统计
#define NEAR_ROI_TOP        (MT9V03X_H - 72)
#define NEAR_ROI_BOT        (MT9V03X_H - 4)

#define HALF_W              (0.5f * (float)MT9V03X_W)

// 近场基准行：从画面最下往上逐 6 行找，第一条能量到完整白段的行作为基准。
// 写死行号在有车头/线束遮挡时会整体失效（仿真：遮挡 46 行即 0/30 全丢）
#define SENSE_SCAN_STEP     6

// 透视：近场半宽 -> 远端半宽的收缩比（越远越窄）
#define PERSP_FAR_RATIO     0.60f

// 逐行连续性：本行边界与上一行相差超过该像素数就丢弃该行
#define CONTINUITY_PX       18

// 前瞻限幅：融合中心相对近场中心的偏移，最多不超过近场半宽的该比例。
// 仿真验证：无限幅时弯道会按远端趋势多打约 19px（≈半个赛道宽），过度切内线
#define LEAD_LIMIT_RATIO    0.35f

volatile uint32_t g_tick_ms = 0;

// 近场大津法：只统计视野下部，输出阈值、前后景对比度、白板像素占比
static uint8_t get_near_otsu (
    uint8_t image[][MT9V03X_W],
    int *out_contrast,
    int *out_white_pct
) {
    int histogram[256] = {0};
    int total_pixels = 0;

    for (int i = NEAR_ROI_TOP; i < NEAR_ROI_BOT; i++) {
        for (int j = 0; j < MT9V03X_W; j++) {
            histogram[image[i][j]]++;
            total_pixels++;
        }
    }

    if (total_pixels <= 0) {
        if (out_contrast) *out_contrast = 0;
        if (out_white_pct) *out_white_pct = 0;
        return 0;
    }

    float sum = 0.0f;
    for (int t = 0; t < 256; t++) {
        sum += (float)(t * histogram[t]);
    }

    float sumB = 0.0f;
    int wB = 0;
    int wF = 0;
    float varMax = -1.0f;
    float best_diff = 0.0f;
    uint8_t threshold = 0;

    for (int t = 0; t < 256; t++) {
        wB += histogram[t];
        if (wB == 0) {
            continue;
        }
        wF = total_pixels - wB;
        if (wF == 0) {
            break;
        }
        sumB += (float)(t * histogram[t]);
        float mB = sumB / (float)wB;
        float mF = (sum - sumB) / (float)wF;
        float diff = mF - mB;
        float varBetween = (float)wB * (float)wF * diff * diff;
        if (varBetween > varMax) {
            varMax = varBetween;
            best_diff = diff;
            threshold = (uint8_t)t;
        }
    }

    int white = 0;
    for (int t = (int)threshold + 1; t < 256; t++) {
        white += histogram[t];
    }

    if (out_contrast) *out_contrast = (int)best_diff;
    if (out_white_pct) *out_white_pct = (white * 100) / total_pixels;
    return threshold;
}

// 单行找白段。
// seed >= 0 时从 seed 附近起步（左右各找 20px 内的白点）；
// seed < 0 时全行扫描，返回最长白段。
// 判“边”的规则：连续 3 个暗像素才认为是赛道外的暗区。
// 白带上的拼缝/胶带只有 1~2px 宽，会被这里过滤掉，不会把道宽截断。
static int scan_row_seg (
    const uint8_t *rowp,
    int n,
    int seed,
    int *out_l,
    int *out_r,
    int *out_lf,
    int *out_rf
) {
    int l = 0;
    int r = n - 1;
    int lf = 0;
    int rf = 0;

    if (seed >= 0) {
        // 起点落在暗区时，在左右 20px 内找最近的白点
        if (rowp[seed] == 0) {
            int found = 0;
            for (int off = 1; off <= 20 && !found; off++) {
                int rx = seed + off;
                int lx = seed - off;
                if (rx < n && rowp[rx] == 255) {
                    seed = rx;
                    found = 1;
                } else if (lx >= 0 && rowp[lx] == 255) {
                    seed = lx;
                    found = 1;
                }
            }
            if (!found) {
                return 0;
            }
        }

        int dark = 0;
        for (int x = seed; x >= 0; x--) {
            if (rowp[x] == 0) {
                if (++dark >= 3) {
                    l = x + 3;
                    lf = 1;
                    break;
                }
            } else {
                dark = 0;
            }
        }
        dark = 0;
        for (int x = seed; x < n; x++) {
            if (rowp[x] == 0) {
                if (++dark >= 3) {
                    r = x - 3;
                    rf = 1;
                    break;
                }
            } else {
                dark = 0;
            }
        }
        if (r < l) {
            return 0;
        }
        *out_l = l;
        *out_r = r;
        *out_lf = lf;
        *out_rf = rf;
        return r - l + 1;
    }

    // 全行最长白段
    int best = 0;
    int bl = 0;
    int br = 0;
    int x = 0;
    while (x < n) {
        if (rowp[x] == 0) {
            x++;
            continue;
        }
        int s = x;
        int e = x;
        int dark = 0;
        for (; x < n; x++) {
            if (rowp[x] == 255) {
                e = x;
                dark = 0;
            } else {
                if (++dark >= 3) {
                    break;
                }
            }
        }
        int len = e - s + 1;
        if (len > best) {
            best = len;
            bl = s;
            br = e;
        }
        x++;
    }
    if (best <= 0) {
        return 0;
    }
    *out_l = bl;
    *out_r = br;
    *out_lf = (bl > 0);
    *out_rf = (br < n - 1);
    return best;
}

static void drive_stop (void) {
    pwm_set_duty (servo, servo_center);
    pwm_set_duty (motor1_in1, 0);
    pwm_set_duty (motor1_in2, 0);
    pwm_set_duty (motor2_in1, 0);
    pwm_set_duty (motor2_in2, 0);
}

// 丢线滑行：舵机保持上一次打角，电机给一小截占空比，避免“断电-满占空比”反复冲击
static void drive_creep (void) {
    int creep = base_speed * LOST_CREEP_PCT / 100;
    pwm_set_duty (motor1_in1, 0);
    pwm_set_duty (motor1_in2, (uint32)creep);
    pwm_set_duty (motor2_in1, 0);
    pwm_set_duty (motor2_in2, (uint32)creep);
}

static int clamp_int (int val, int lo, int hi) {
    if (val < lo) return lo;
    if (val > hi) return hi;
    return val;
}

static float clamp_float (float val, float lo, float hi) {
    if (val < lo) return lo;
    if (val > hi) return hi;
    return val;
}

static int iabs (int v) {
    return v < 0 ? -v : v;
}

// 电机占空比斜坡，避免 0 <-> 3000 瞬间跳变造成车体顿挫
static int speed_ramp (int cur, int target) {
    if (target > cur + SPEED_RAMP_STEP) return cur + SPEED_RAMP_STEP;
    if (target < cur - SPEED_RAMP_STEP) return cur - SPEED_RAMP_STEP;
    return target;
}

// 近场半宽 -> 该扫描行的预期半宽（线性透视收缩），基准行随遮挡自适应，需作为参数传入
static float persp_half (float near_half, int base_row, int row) {
    int den = base_row - SCAN_TOP_ROW;
    if (den < 1) {
        den = 1;
    }
    float t = ((float)base_row - (float)row) / (float)den;
    t = clamp_float (t, 0.0f, 1.0f);
    return near_half * (1.0f - (1.0f - PERSP_FAR_RATIO) * t);
}

int main (void) {
    clock_init (SYSTEM_CLOCK_120M);
    debug_init();
    mt9v03x_init();
    tft180_init();
    pit_ms_init (TIM6_PIT, 10);
    pwm_init (servo, 50, servo_center);
    pwm_init (motor1_in1, 17000, 0);
    pwm_init (motor1_in2, 17000, 0);
    pwm_init (motor2_in1, 17000, 0);
    pwm_init (motor2_in2, 17000, 0);
    drive_stop();
    tft180_show_string (0, 0, "WAIT CAM     ");
    tft180_show_string (0, 64, "n=0     ");

    uint8_t otsu_T = 0;
    float image_error = 0.0f;
    uint8_t image_valid = 0;
    float filt_error = 0.0f;
    float prev_error = 0.0f;
    float last_valid_error = 0.0f;
    uint32_t last_frame_tick = 0;
    uint32_t frame_cnt = 0;
    int lost_frames = 0;
    int speed_cmd = 0;
    uint8_t state_drawn = 0xFF;     // 0=LOST 1=RUN 2=CROSS 3=HOLD
    float s_near_half = 55.0f;      // 赛道近场半宽的滑动估计（像素），由实测驱动
    int s_near_w = 0;               // 上一帧近场道宽，用于丢帧兜底

    while (1) {
        // step 1 检测入口：等待摄像头新帧；超时无帧按丢线停车
        if (!mt9v03x_finish_flag) {
            if (image_valid && (g_tick_ms - last_frame_tick > FRAME_TIMEOUT_MS)) {
                image_valid = 0;
                lost_frames = LOST_HOLD_FRAMES + 1;
                speed_cmd = 0;
                drive_stop();
                if (state_drawn != 0) {
                    tft180_show_string (0, 0, "LOST         ");
                    state_drawn = 0;
                }
            }
            continue;
        }
        mt9v03x_finish_flag = 0;
        last_frame_tick = g_tick_ms;
        frame_cnt++;

        // step 2 二值化：近场 ROI 大津阈值；白占比校验“画面里确实有白带”
        int otsu_contrast = 0;
        int white_pct = 0;
        otsu_T = get_near_otsu (mt9v03x_image, &otsu_contrast, &white_pct);
        for (int i = 0; i < MT9V03X_H; i++) {
            for (int j = 0; j < MT9V03X_W; j++) {
                mt9v03x_image[i][j] = (mt9v03x_image[i][j] > otsu_T) ? 255 : 0;
            }
        }

        // step 3.1 近场基准行：从下往上找第一条能量到完整白段的行，同时实测赛道像素宽度
        int near_w = 0;
        int near_l = 0;
        int near_r = MT9V03X_W - 1;
        int base_row = SCAN_BOT_ROW;
        for (int row = SCAN_BOT_ROW; row > SCAN_TOP_ROW; row -= SENSE_SCAN_STEP) {
            int l, r, lf, rf;
            int w = scan_row_seg (mt9v03x_image[row], MT9V03X_W, MT9V03X_W / 2, &l, &r, &lf, &rf);
            if (w >= NEAR_SEG_MIN_W) {
                near_w = w;
                near_l = l;
                near_r = r;
                base_row = row;
                break;
            }
        }
        // 中心落进蓝区（车已偏出赛道）时，退化为全行最长白段
        if (near_w == 0) {
            for (int row = SCAN_BOT_ROW; row > SCAN_TOP_ROW; row -= SENSE_SCAN_STEP) {
                int l, r, lf, rf;
                int w = scan_row_seg (mt9v03x_image[row], MT9V03X_W, -1, &l, &r, &lf, &rf);
                if (w >= NEAR_SEG_MIN_W) {
                    near_w = w;
                    near_l = l;
                    near_r = r;
                    base_row = row;
                    break;
                }
            }
        }

        if (near_w >= NEAR_SEG_MIN_W) {
            s_near_w = near_w;
            // 慢学习：避免单帧异常（贴边、横叉）把尺度带跑
            float meas_half = 0.5f * (float)near_w;
            meas_half = clamp_float (meas_half, 26.0f, 92.0f);
            s_near_half += 0.30f * (meas_half - s_near_half);
        } else {
            near_w = s_near_w;
        }
        float near_half = clamp_float (s_near_half, 26.0f, 92.0f);
        int near_center = (near_l + near_r) / 2;

        // step 3.2 从近到远逐行跟踪，带连续性约束
        float wsum = 0.0f;
        float wtot = 0.0f;
        int valid_cnt = 0;
        int cross_rows = 0;
        int scan_cnt = 0;

        int prev_l = near_l;
        int prev_r = near_r;
        int have_prev = (near_w >= NEAR_SEG_MIN_W);

        for (int i = 0; i < SCAN_ROWS; i++) {
            int row = base_row - i * SCAN_STEP;
            if (row <= SCAN_TOP_ROW) {
                break;
            }
            scan_cnt++;

            int seed = have_prev ? ((prev_l + prev_r) / 2) : (MT9V03X_W / 2);
            seed = clamp_int (seed, 1, MT9V03X_W - 2);

            int l, r, lf, rf;
            int w = scan_row_seg (mt9v03x_image[row], MT9V03X_W, seed, &l, &r, &lf, &rf);
            if (w < 8) {
                continue;
            }

            float half_exp = persp_half (near_half, base_row, row);
            float max_ok_w = 2.4f * half_exp;

            // 近场优先权重：车头正前方才是当前姿态，远端只用来看弯道趋势
            float weight;
            if (row >= 100) {
                weight = 2.2f;
            } else if (row >= 88) {
                weight = 1.6f;
            } else if (row >= 74) {
                weight = 0.8f;
            } else {
                weight = 0.25f;
            }

            if (lf && rf) {
                // 双边都有：最可信
                if ((float)w > max_ok_w) {
                    continue;   // 明显超宽（横叉/十字横道），不用于定中心
                }
                if (have_prev && (iabs(l - prev_l) > CONTINUITY_PX || iabs(r - prev_r) > CONTINUITY_PX)) {
                    continue;   // 与上一行跳变过大：白带上的污渍或跨到别的赛道段
                }
                float c = 0.5f * (float)(l + r);
                wsum += c * weight;
                wtot += weight;
                valid_cnt++;
                prev_l = l;
                prev_r = r;
                have_prev = 1;
            } else if (lf && !rf) {
                // 右边界出画（向右急弯）：按该行预期半宽外推
                float c = (float)l + half_exp;
                wsum += c * (weight * 0.7f);
                wtot += weight * 0.7f;
                valid_cnt++;
                prev_l = l;
                prev_r = (int)((float)l + 2.0f * half_exp);
                have_prev = 1;
            } else if (!lf && rf) {
                // 左边界出画（向左急弯）
                float c = (float)r - half_exp;
                wsum += c * (weight * 0.7f);
                wtot += weight * 0.7f;
                valid_cnt++;
                prev_r = r;
                prev_l = (int)((float)r - 2.0f * half_exp);
                have_prev = 1;
            } else {
                // 左右都没有连续暗区：整行全白，典型十字横道特征
                cross_rows++;
            }
        }

        // step 3.3 中心融合
        uint8_t is_cross = (scan_cnt > 0
                            && (float)cross_rows >= CROSS_ROWS_RATIO * (float)scan_cnt);
        float center_x = (float)near_center;
        if (valid_cnt >= 3 && wtot > 0.0f) {
            center_x = wsum / wtot;
        } else if (valid_cnt >= 1 && wtot > 0.0f) {
            // 有效行太少：近场基准与实际跟踪各占一半，不信任单点
            center_x = 0.5f * (float)near_center + 0.5f * (wsum / wtot);
        }
        if (is_cross) {
            // 身处十字正中：保留少量进十字前的误差趋势，平稳直行
            center_x = HALF_W + 0.30f * last_valid_error;
        } else {
            // 前瞻限幅：允许按远端趋势提前打舵，但不许超过近场半宽的 LEAD_LIMIT_RATIO 倍，
            // 否则急弯会按远端趋势过度切内线（仿真实测可多打约 19px）
            float lead = center_x - (float)near_center;
            float lim = LEAD_LIMIT_RATIO * near_half;
            if (lead > lim) {
                center_x = (float)near_center + lim;
            } else if (lead < -lim) {
                center_x = (float)near_center - lim;
            }
        }
        center_x = clamp_float (center_x, -HALF_W, 1.5f * (float)MT9V03X_W);

        // 有效性：有点像样的前后景对比 + 近场确实量到白段 + 找到了线。
        // 不再用整块 ROI 的白占比：车头/线束遮挡会让 ROI 内大片为黑，白占比被拉低后误判丢线
        // （仿真：遮挡 46 行时白占比只剩 12%，old 判据直接 0/30 全失效，near_w 却是正常的 59）
        uint8_t image_ok = (otsu_contrast >= MIN_CONTRAST
                            && near_w >= NEAR_SEG_MIN_W
                            && (valid_cnt > 0 || is_cross));

        // step 4 闭环控制：按实测半宽归一化 PD + 弯道自减速 + 丢线保持
        if (image_ok) {
            lost_frames = 0;
            image_error = center_x - HALF_W;
            image_valid = 1;
            if (!is_cross && valid_cnt >= 2) {
                last_valid_error = image_error;
            }

            filt_error += ERROR_LPF_ALPHA * (image_error - filt_error);

            // 连续死区：死区边界处从 0 平滑接上，不留阶跃
            float abs_err = filt_error >= 0.0f ? filt_error : -filt_error;
            float eff_error = 0.0f;
            float dz = DEADBAND_PX;
            if (abs_err > dz) {
                float k = (abs_err - dz) / dz;
                if (k > 1.0f) k = 1.0f;
                eff_error = filt_error * k;
            }

            // 归一化：偏差达到半个实测赛道宽才打满舵，不再 50px 就打满
            float norm_e = clamp_float (eff_error / near_half, -1.8f, 1.8f);
            float d_norm = clamp_float ((eff_error - prev_error) / near_half, -1.8f, 1.8f);
            prev_error = eff_error;

            float d_term = clamp_float (KD_NORM * d_norm, -D_TERM_LIMIT, D_TERM_LIMIT);
            float servo_turn = (KP_NORM * norm_e + d_term) * (float)MAX_SERVO_TURN * (float)SERVO_DIR;
            servo_turn = clamp_float (servo_turn, -(float)MAX_SERVO_TURN, (float)MAX_SERVO_TURN);
            pwm_set_duty (servo, (uint32)(servo_center + (int)servo_turn));

            float absn = norm_e >= 0.0f ? norm_e : -norm_e;
            if (absn > 1.0f) absn = 1.0f;
            int target_speed = (int)((float)base_speed * (1.0f - TURN_SLOW_RATIO * absn));
            if (target_speed < turn_min_speed) target_speed = turn_min_speed;
            if (target_speed > max_speed) target_speed = max_speed;
            speed_cmd = speed_ramp (speed_cmd, target_speed);
            pwm_set_duty (motor1_in1, 0);
            pwm_set_duty (motor1_in2, (uint32)speed_cmd);
            pwm_set_duty (motor2_in1, 0);
            pwm_set_duty (motor2_in2, (uint32)speed_cmd);
        } else {
            // 丢线不再立刻断电：短时间内保持舵机 + 低速滑行
            if (lost_frames < 250) {
                lost_frames++;
            }
            if (lost_frames <= LOST_HOLD_FRAMES) {
                drive_creep();
            } else {
                image_valid = 0;
                speed_cmd = 0;
                // 不清 filt_error/prev_error：恢复瞬间不再出现微分项阶跃冲击
                drive_stop();
            }
        }

        // step 5 降频状态显示
        uint8_t refresh_now = (frame_cnt == 1 || (frame_cnt % TFT_REFRESH_DIV == 0));
        uint8_t state_now;
        if (image_valid && image_ok) {
            state_now = is_cross ? 2 : 1;
        } else if (lost_frames <= LOST_HOLD_FRAMES) {
            state_now = 3;
        } else {
            state_now = 0;
        }

        if (state_now != state_drawn || (refresh_now && state_now != 0)) {
            if (state_now == 0) {
                tft180_show_string (0, 0, "LOST         ");
            } else if (state_now == 3) {
                tft180_show_string (0, 0, "HOLD         ");
            } else if (state_now == 2) {
                tft180_show_string (0, 0, "CROSS        ");
            } else {
                float abs_err = image_error >= 0.0f ? image_error : -image_error;
                if (abs_err < CENTER_OK_RATIO * near_half) {
                    tft180_show_string (0, 0, "1111111111111");
                } else {
                    tft180_show_string (0, 0, "33333333333  ");
                }
            }
            state_drawn = state_now;
        }

        if (refresh_now) {
            tft180_show_string (0, 16, "C=");
            tft180_show_uint (16, 16, (uint32)(otsu_contrast > 0 ? otsu_contrast : 0), 3);
            tft180_show_string (48, 16, "P=");
            tft180_show_uint (64, 16, (uint32)white_pct, 3);
            tft180_show_string (0, 32, "W=");
            tft180_show_uint (16, 32, (uint32)near_w, 3);
            tft180_show_string (48, 32, "v=");
            tft180_show_uint (64, 32, (uint32)valid_cnt, 2);
            tft180_show_string (80, 32, "h=");
            tft180_show_uint (96, 32, (uint32)((int)near_half), 2);
            tft180_show_string (0, 48, "e=");
            tft180_show_int (16, 48, (int)image_error, 4);
            tft180_show_string (56, 48, "x=");
            tft180_show_uint (72, 48, (uint32)cross_rows, 2);
            tft180_show_string (0, 64, "n=");
            tft180_show_uint (16, 64, frame_cnt, 6);
        }
    }
}
