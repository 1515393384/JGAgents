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
 * 主循环 main step（while 1 内顺序执行）：
 * step 1 检测入口：等摄像头新帧；100ms 无帧按丢线停车
 * step 2 二值化：近场 ROI 大津阈值 + 近场白占比校验（远端背景不再把阈值带跑）
 * step 3 找线：抗板缝种子列 -> 远近逐行跟踪 -> 单边弯道外推 -> 加权中心
 * step 4 闭环控制：归一化 PD（不随像素尺度提前饱和）+ 弯道自减速 + 丢线保持
 * step 5 状态显示：每 10 帧刷新 TFT 遥测
 *
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

#define KP_NORM             1.60f   // 归一化转向比例（弯道欠增益会在外侧贴边，见下方注释）
#define KD_NORM             0.28f   // 归一化转向阻尼
#define D_TERM_LIMIT        0.35f   // 微分项最多占总打角的比例（防恢复帧冲击）
#define ERROR_LPF_ALPHA     0.55f   // 误差低通滤波系数
#define DEADBAND_PX         2.5f    // 中心小死区（像素），死区内线性收敛不留阶跃

#define base_speed          1200    // 直道基础占空比（满分 10000）；差速跑稳后再一档档往上加
#define turn_min_speed      900     // 急弯最低保底占空比（防弯道降速过多堵转）
#define max_speed           2200    // 最大限速保护
#define TURN_SLOW_GAIN      7.0f    // 弯道减速系数：误差越大适当减速（原来 4.5 太弱，急弯来不及收速）
#define SPEED_RAMP_STEP     90      // 电机占空比每帧最大变化量（消除停-走冲击）

// ── 图像识别参数（适配 45.5cm 白底蓝垫跑道）────────────────────────────────────
#define MIN_CONTRAST        12      // 近场前后景最小灰度差（放宽以兼容偏暗室内光）
#define NEAR_WHITE_MIN_PCT  20      // 近场白板像素占比下限（%），低于此判为没看到赛道
#define CROSS_ROWS_RATIO    0.55f   // 无边行占比超过该值判为十字
#define CENTER_OK_RATIO     0.08f   // 屏幕显示 1111 居中的误差占比阈值
#define FRAME_TIMEOUT_MS    100
#define TFT_REFRESH_DIV     10      // 每 10 帧刷一次屏幕（首帧与状态切换时立即刷新）
#define LOST_HOLD_FRAMES    6       // 连续丢线容忍帧数，之内保持舵机滑行
#define LOST_CREEP_PCT      45      // 丢线滑行占基础速度的百分比

// ── 电子差速（左右轮独立 PWM，没有机械差速器）──────────────────────────────────
// 两轮被强行走同样距离时，弯里只能靠轮胎打滑硬拐，速度越快越推头。
// 这里在弯道让内轮减速、外轮加速，等于软件做出差速效果。
#define DIFF_ENABLE         1       // 0 = 关闭，恢复左右同速
#define DIFF_GAIN           0.30f   // 满幅误差时内外轮的速度差比例（0.3 = 差 30%）
#define DIFF_DIR            1       // 差速方向：若弯里反而更转不过来，改成 -1

#define SCAN_BOT_ROW        (MT9V03X_H - 10)
#define SCAN_TOP_ROW        40
#define SCAN_STEP           3
#define SCAN_ROWS           (((SCAN_BOT_ROW - SCAN_TOP_ROW) / SCAN_STEP) + 1)

// 近场 ROI：视野下部，赛道一定在这里；远端背景（蓝垫/看台）不参与统计
#define NEAR_ROI_TOP        (MT9V03X_H - 72)
#define NEAR_ROI_BOT        (MT9V03X_H - 4)

#define HALF_W              (0.5f * (float)MT9V03X_W)

volatile uint32_t g_tick_ms = 0;

// 近场大津法：只统计视野下部，输出阈值、前后景对比度、白板像素占比
// 远端在弯道上会被赛道外背景填满，全幅统计会让阈值随弯道漂移 -> 整帧判定翻转
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

    if (out_contrast) {
        *out_contrast = (int)best_diff;
    }
    if (out_white_pct) {
        *out_white_pct = (white * 100) / total_pixels;
    }
    return threshold;
}

static void drive_stop (void) {
    pwm_set_duty (servo, servo_center);
    pwm_set_duty (motor1_in1, 0);
    pwm_set_duty (motor1_in2, 0);
    pwm_set_duty (motor2_in1, 0);
    pwm_set_duty (motor2_in2, 0);
}

// 丢线滑行：舵机保持上一次打角，电机只给一小截占空比，避免“断电-满占空比”反复冲击
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

// 电机占空比斜坡，避免 0 <-> 1350 瞬间跳变造成车体顿挫
static int speed_ramp (int cur, int target) {
    if (target > cur + SPEED_RAMP_STEP) return cur + SPEED_RAMP_STEP;
    if (target < cur - SPEED_RAMP_STEP) return cur - SPEED_RAMP_STEP;
    return target;
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

    // 每一扫描行的经验半宽（只从真实双边行学习，解决近宽远窄透视误差）
    static float s_row_half_w[SCAN_ROWS];
    for (int i = 0; i < SCAN_ROWS; i++) {
        // 先验：近端(i=0)半宽约 62px，远端(i=max)半宽约 32px
        s_row_half_w[i] = 62.0f - (30.0f * (float)i) / (float)(SCAN_ROWS - 1);
    }

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
    uint8_t state_drawn = 0xFF;     // 0=LOST 1=RUN 2=CROSS 3=HOLD，用于只在变化时刷屏

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

        // step 2 二值化：近场 ROI 大津阈值；白占比作为“有没有看到白板”的判据
        int otsu_contrast = 0;
        int white_pct = 0;
        otsu_T = get_near_otsu (mt9v03x_image, &otsu_contrast, &white_pct);
        for (int i = 0; i < MT9V03X_H; i++) {
            for (int j = 0; j < MT9V03X_W; j++) {
                mt9v03x_image[i][j] = (mt9v03x_image[i][j] > otsu_T) ? 255 : 0;
            }
        }

        // step 3 找线：
        // 3.1 抗板缝 & 避车头种子列：先跳过底部可能拍到的黑色车头/轮胎，再统计最长白列（允许跨越 <=2 行黑缝）
        int col_len[MT9V03X_W];
        int max_white_len = 0;
        for (int x = 2; x < MT9V03X_W - 2; x++) {
            int y = MT9V03X_H - 4;
            while (y >= (MT9V03X_H / 2) && mt9v03x_image[y][x] == 0) {
                y--;
            }
            int len = 0;
            int dark_gap = 0;
            for (; y >= 0; y--) {
                if (mt9v03x_image[y][x] == 255) {
                    len += 1 + dark_gap;
                    dark_gap = 0;
                } else {
                    dark_gap++;
                    if (dark_gap > 2) {
                        break;
                    }
                }
            }
            col_len[x] = len;
            if (len > max_white_len) {
                max_white_len = len;
            }
        }

        int seed_sum = 0;
        int seed_cnt = 0;
        if (max_white_len > 0) {
            int len_floor = max_white_len > 6 ? (max_white_len - 5) : 1;
            for (int x = 2; x < MT9V03X_W - 2; x++) {
                if (col_len[x] >= len_floor) {
                    seed_sum += x;
                    seed_cnt++;
                }
            }
        }
        int start_x = seed_cnt > 0 ? (seed_sum / seed_cnt) : (MT9V03X_W / 2);

        // 3.2 从近到远逐行跟踪搜边（允许每行相对上一行轻微摆动，跳过横向黑缝）
        float both_w_sum = 0.0f;
        float both_weight_sum = 0.0f;
        int both_cnt = 0;

        float all_w_sum = 0.0f;
        float all_weight_sum = 0.0f;
        int valid_cnt = 0;
        int edge_missing_rows = 0;  // 整行左右都没有黑边（真正的十字特征）
        int mid_width = 0;

        int track_x = start_x;
        for (int i = 0; i < SCAN_ROWS; i++) {
            int row = SCAN_BOT_ROW - i * SCAN_STEP;
            if (row < 1) {
                break;
            }

            // 若当前 track_x 恰好踩中板缝或车头噪点，在其左右 +/-30px 内寻找最近白点
            int row_seed = track_x;
            if (mt9v03x_image[row][row_seed] == 0) {
                int found = 0;
                for (int off = 1; off <= 30; off++) {
                    int rx = track_x + off;
                    int lx = track_x - off;
                    if (rx < MT9V03X_W - 1 && mt9v03x_image[row][rx] == 255) {
                        row_seed = rx;
                        found = 1;
                        break;
                    }
                    if (lx > 0 && mt9v03x_image[row][lx] == 255) {
                        row_seed = lx;
                        found = 1;
                        break;
                    }
                }
                if (!found) {
                    // 整行近处都是黑（板缝黑行 / 车头遮挡 / 越过远端视野），不计入十字特征
                    continue;
                }
            }

            int l = 0;
            int r = MT9V03X_W - 1;
            uint8_t lf = 0;
            uint8_t rf = 0;

            for (int x = row_seed; x >= 1; x--) {
                if (mt9v03x_image[row][x] == 0 && mt9v03x_image[row][x - 1] == 0) {
                    l = x + 1;
                    lf = 1;
                    break;
                }
            }
            for (int x = row_seed; x < MT9V03X_W - 1; x++) {
                if (mt9v03x_image[row][x] == 0 && mt9v03x_image[row][x + 1] == 0) {
                    r = x - 1;
                    rf = 1;
                    break;
                }
            }

            float expected_w = s_row_half_w[i] * 2.0f;
            float max_valid_w = expected_w * 1.45f;
            if (max_valid_w > (float)(MT9V03X_W - 12)) {
                max_valid_w = (float)(MT9V03X_W - 12);
            }

            // 前瞻权重：中近段(row 85~45)权重最高，兼顾居中稳定性与入弯提前量
            float weight = 1.0f;
            if (row <= 92 && row >= 44) {
                weight = 1.6f;
            } else if (row < 44) {
                weight = 1.0f;
            }

            if (lf && rf) {
                int w = r - l;
                if (w < 8) {
                    // 极窄白隙通常是黑缝边缘残片，跳过
                    continue;
                }
                if ((float)w > max_valid_w) {
                    // 宽度超限只丢弃这一行，不算十字：宽度估计有偏差时不会把直道判成十字
                    continue;
                }

                float c = 0.5f * (float)(l + r);
                both_w_sum += c * weight;
                both_weight_sum += weight;
                both_cnt++;

                all_w_sum += c * weight;
                all_weight_sum += weight;
                valid_cnt++;

                // 在线平滑更新本行真实半宽（慢学习，避免单帧异常把门限带偏）
                float meas_half = 0.5f * (float)w;
                if (meas_half >= 14.0f && meas_half <= 92.0f) {
                    s_row_half_w[i] += 0.05f * (meas_half - s_row_half_w[i]);
                }
                if (row >= 75 && row <= 85) {
                    mid_width = w;
                } else if (mid_width == 0) {
                    mid_width = w;
                }
                track_x = clamp_int ((int)c, 2, MT9V03X_W - 3);
            } else if (!lf && !rf) {
                // 左右同时全白到达图像边界：典型十字横道
                edge_missing_rows++;
            } else if (lf && !rf) {
                int vis_w = (MT9V03X_W - 1) - l;
                if ((float)vis_w > max_valid_w) {
                    // 单侧已超出正常道宽并延展到右边界：不按右弯外推，丢弃该行
                    continue;
                }
                if (l < MT9V03X_W - 8) {
                    // 右丢线（向右急弯）：按该行经验半宽向右侧外推真实中心
                    float c = (float)l + s_row_half_w[i];
                    all_w_sum += c * weight;
                    all_weight_sum += weight;
                    valid_cnt++;
                    track_x = clamp_int ((l + (MT9V03X_W - 1)) / 2, 2, MT9V03X_W - 3);
                }
            } else {
                int vis_w = r;
                if ((float)vis_w > max_valid_w) {
                    // 单侧已超出正常道宽并延展到左边界：不按左弯外推，丢弃该行
                    continue;
                }
                if (r > 7) {
                    // 左丢线（向左急弯）：按该行经验半宽向左侧外推真实中心
                    float c = (float)r - s_row_half_w[i];
                    all_w_sum += c * weight;
                    all_weight_sum += weight;
                    valid_cnt++;
                    track_x = clamp_int (r / 2, 2, MT9V03X_W - 3);
                }
            }
        }

        // 3.3 十字判定与最终中心融合
        uint8_t is_cross = ((float)edge_missing_rows >= CROSS_ROWS_RATIO * (float)SCAN_ROWS);
        float center_x = (float)start_x;
        if (is_cross) {
            if (both_cnt >= 2 && both_weight_sum > 0.0f) {
                // 十字前后仍看得到正常双边时，只用正常双边居中穿越，不误拐进横叉
                center_x = both_w_sum / both_weight_sum;
            } else {
                // 身处十字正中全白区：保留少量进十字前误差趋势平稳直行
                center_x = HALF_W + 0.30f * last_valid_error;
            }
        } else if (valid_cnt > 0 && all_weight_sum > 0.0f) {
            // 弯道/直道：同时融合双边与单边外推前瞻行，不再被底边直道行屏蔽
            center_x = all_w_sum / all_weight_sum;
        }
        center_x = clamp_float (center_x, -HALF_W, 1.5f * (float)MT9V03X_W);

        // 有效性只看“近场有没有白板 + 有没有找到线”，不再被最大白列长度绑架
        uint8_t image_ok = (otsu_contrast >= MIN_CONTRAST
                            && white_pct >= NEAR_WHITE_MIN_PCT
                            && (valid_cnt > 0 || is_cross));

        // step 4 闭环控制：归一化 PD + 弯道自减速 + 丢线保持
        if (image_ok) {
            lost_frames = 0;
            image_error = center_x - HALF_W;
            image_valid = 1;
            if (!is_cross && both_cnt >= 2) {
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

            // 按半幅归一化：只有接近满幅的误差才顶满舵，不再 50px 就打满
            float norm_e = eff_error / HALF_W;
            float d_norm = (eff_error - prev_error) / HALF_W;
            prev_error = eff_error;

            float d_term = clamp_float (KD_NORM * d_norm, -D_TERM_LIMIT, D_TERM_LIMIT);
            float servo_turn = (KP_NORM * norm_e + d_term) * (float)MAX_SERVO_TURN * (float)SERVO_DIR;
            servo_turn = clamp_float (servo_turn, -(float)MAX_SERVO_TURN, (float)MAX_SERVO_TURN);
            pwm_set_duty (servo, (uint32)(servo_center + (int)servo_turn));

            int target_speed = base_speed - (int)(abs_err * TURN_SLOW_GAIN);
            if (target_speed < turn_min_speed) target_speed = turn_min_speed;
            if (target_speed > max_speed) target_speed = max_speed;
            speed_cmd = speed_ramp (speed_cmd, target_speed);
            // 电子差速：norm_e>0 表示赛道中心在画面右侧 → 要向右转 → 右轮内、左轮外
            int sp_l = speed_cmd;
            int sp_r = speed_cmd;
#if DIFF_ENABLE
            {
                float ke = norm_e >= 0.0f ? norm_e : -norm_e;
                float kr = ke * DIFF_GAIN;
                if (kr > 0.45f) kr = 0.45f;
                int dv = (int)((float)speed_cmd * kr);
                if (norm_e > 0.0f) {
                    sp_l = speed_cmd + dv;
                    sp_r = speed_cmd - dv;
                } else if (norm_e < 0.0f) {
                    sp_l = speed_cmd - dv;
                    sp_r = speed_cmd + dv;
                }
                if (sp_l < 0) sp_l = 0;
                if (sp_r < 0) sp_r = 0;
                if (sp_l > max_speed) sp_l = max_speed;
                if (sp_r > max_speed) sp_r = max_speed;
            }
#endif
            if (DIFF_DIR < 0) {
                int t = sp_l;
                sp_l = sp_r;
                sp_r = t;
            }
            pwm_set_duty (motor1_in1, 0);
            pwm_set_duty (motor1_in2, (uint32)sp_l);
            pwm_set_duty (motor2_in1, 0);
            pwm_set_duty (motor2_in2, (uint32)sp_r);
        } else {
            // 丢线不再立刻断电：短时间内保持舵机 + 低速滑行，避免每帧“刹-冲”造成抽搐
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

        // step 5 降频状态显示（状态切换立即刷新，其余每 10 帧刷新遥测）
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
                int ref_w = mid_width > 15 ? mid_width : 80;
                if (abs_err < CENTER_OK_RATIO * (float)ref_w) {
                    tft180_show_string (0, 0, "1111111111111");
                } else {
                    tft180_show_string (0, 0, "33333333333  ");
                }
            }
            state_drawn = state_now;
        }

        if (refresh_now) {
            int disp_c = otsu_contrast > 0 ? otsu_contrast : 0;
            tft180_show_string (0, 16, "C=");
            tft180_show_uint (16, 16, (uint32)disp_c, 3);
            tft180_show_string (48, 16, "P=");
            tft180_show_uint (64, 16, (uint32)white_pct, 3);
            tft180_show_string (0, 32, "W=");
            tft180_show_uint (16, 32, (uint32)mid_width, 3);
            tft180_show_string (48, 32, "L=");
            tft180_show_uint (64, 32, (uint32)max_white_len, 3);
            tft180_show_string (0, 48, "e=");
            tft180_show_int (16, 48, (int)image_error, 4);
            tft180_show_string (56, 48, "v=");
            tft180_show_uint (72, 48, (uint32)valid_cnt, 2);
            tft180_show_string (0, 64, "n=");
            tft180_show_uint (16, 64, frame_cnt, 6);
        }
    }
}
