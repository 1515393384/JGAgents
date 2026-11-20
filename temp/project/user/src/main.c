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
 * step 2 二值化：全图大津阈值 + 全图黑白化
 * step 3 找线：种子列 -> 全宽搜左右边 -> 边缺省回种子列；种子白长>=5 才有效
 * step 4 直线闭环：有效帧 PD 纠偏 + 差速直行；无效帧立即停车
 * step 5 状态显示：1111 居中 / 333 纠正中 / LOST 停车
 *
 * 函数功能节点：
 * get_otsu_threshold ：下半幅直方图 -> 类间方差遍历 -> 返回二值化阈值
 * drive_stop         ：舵机回中 + 四路电机清零（上电/丢线/超时共用停车节点）
 * main               ：外设初始化 + while 主循环（step 1~5）
 */
#include "zf_common_headfile.h"
#define servo TIM2_PWM_MAP1_CH1_A15
#define motor1_in1 TIM4_PWM_MAP1_CH1_D12
#define motor1_in2 TIM4_PWM_MAP1_CH2_D13
#define motor2_in1 TIM1_PWM_MAP3_CH2_E11
#define motor2_in2 TIM1_PWM_MAP3_CH1_E9

#define servo_center 750
#define SERVO_DIR 1
#define kp 3.8f
#define kd 1.8f
#define ERROR_LPF_ALPHA 0.7f
#define straight_speed 1450
#define curve_speed 1150
#define max_speed 2200
#define MOTOR_SCALE 2.0f
#define MAX_MOTOR_TURN 500
#define MAX_SERVO_TURN 150
#define CENTER_OK_RATIO 0.1f
#define FRAME_TIMEOUT_MS 100

volatile uint32_t g_tick_ms = 0;

// 二值化阈值节点：直方图统计 -> 类间方差遍历 -> 取方差最大处阈值
// 调用方传全幅图像，与 bak 验证过的形态一致
uint8_t get_otsu_threshold (uint8_t image[][MT9V03X_W], int width, int height, int *out_contrast) {
    int histogram[256] = {0};
    int start_y = height / 3;
    int total_pixels = width * (height - start_y);
    float sum = 0;

    for (int i = start_y; i < height; i++) {
        for (int j = 0; j < width; j++) {
            histogram[image[i][j]]++;
            sum += image[i][j];
        }
    }

    float sumB = 0;
    int wB = 0;
    int wF = 0;
    float varMax = 0;
    float best_diff = 0;
    uint8_t threshold = 0;

    for (int t = 0; t < 256; t++) {
        wB += histogram[t];
        if (wB == 0)
            continue;
        wF = total_pixels - wB;
        if (wF == 0)
            break;
        sumB += (float)(t * histogram[t]);
        float mB = sumB / wB;
        float mF = (sum - sumB) / wF;
        float diff = mF - mB;
        float varBetween = (float)wB * (float)wF * diff * diff;
        if (varBetween > varMax) {
            varMax = varBetween;
            best_diff = diff;
            threshold = t;
        }
    }
    if (out_contrast) {
        *out_contrast = (int)best_diff;
    }
    return threshold;
}

// 停车节点：舵机回中 + 四路电机 PWM 清零
// 调用点：上电初始化、step 1 无帧超时、step 4 无效帧
static void drive_stop (void) {
    pwm_set_duty (servo, servo_center);
    pwm_set_duty (motor1_in1, 0);
    pwm_set_duty (motor1_in2, 0);
    pwm_set_duty (motor2_in1, 0);
    pwm_set_duty (motor2_in2, 0);
}

// 初始化节点：时钟/调试/摄像头/屏幕/TIM6 心跳/舵机与电机 PWM，先停车再进循环
// 循环体按 step 1~5 执行，总览见文件头备注块
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
    tft180_show_string(0, 0, "LOST         ");

    uint8_t otsu_T = 0;
    float image_error = 0;
    uint8_t image_valid = 0;
    float filt_error = 0;
    float prev_error = 0;
    uint32_t last_frame_tick = 0;
    static int s_last_width = MT9V03X_W / 2;

    while (1) {
        // step 1 检测入口：等待摄像头新帧；超时无帧按丢线停车
        if (!mt9v03x_finish_flag) {
            if (image_valid && g_tick_ms - last_frame_tick > FRAME_TIMEOUT_MS) {
                image_valid = 0;
                filt_error = 0;
                prev_error = 0;
                drive_stop();
                tft180_show_string(0, 0, "LOST         ");
            }
            continue;
        }
        mt9v03x_finish_flag = 0;
        last_frame_tick = g_tick_ms;

        // step 2 二值化：全图大津阈值 + 全图黑白化
        int otsu_contrast = 0;
        otsu_T = get_otsu_threshold (mt9v03x_image, MT9V03X_W, MT9V03X_H, &otsu_contrast);
        for (int i = 0; i < MT9V03X_H; i++) {
            for (int j = 0; j < MT9V03X_W; j++) {
                if (mt9v03x_image[i][j] > otsu_T) {
                    mt9v03x_image[i][j] = 255;
                } else {
                    mt9v03x_image[i][j] = 0;
                }
            }
        }

        // step 3 找线：种子列 + 全宽搜边；边找不齐缺省回种子列（bak 形态），全黑无道才无效
        int max_white_len = 0;
        int seed_sum = 0;
        int seed_cnt = 0;
        for (int x = 0; x < MT9V03X_W; x++) {
            int current_len = 0;
            for (int y = MT9V03X_H - 1; y >= 0; y--) {
                if (mt9v03x_image[y][x] == 255) {
                    current_len++;
                } else {
                    break;
                }
            }
            if (current_len > max_white_len) {
                max_white_len = current_len;
                seed_sum = x;
                seed_cnt = 1;
            } else if (current_len == max_white_len && max_white_len > 0) {
                seed_sum += x;
                seed_cnt++;
            }
        }
        int start_x = seed_cnt > 0 ? (seed_sum / seed_cnt) : (MT9V03X_W / 2);

        int both_center_sum = 0;
        int both_width_sum = 0;
        int both_cnt = 0;
        int fallback_sum = 0;
        int fallback_cnt = 0;

        int both_weight_sum = 0;

        for (int row = MT9V03X_H - 20; row >= MT9V03X_H - 65; row -= 5) {
            if (row < 0 || mt9v03x_image[row][start_x] == 0) {
                continue;
            }
            int l = 0;
            int r = MT9V03X_W - 1;
            uint8_t lf = 0;
            uint8_t rf = 0;
            for (int x = start_x; x >= 1; x--) {
                if (mt9v03x_image[row][x] == 0 && mt9v03x_image[row][x - 1] == 0) {
                    l = x + 1;
                    lf = 1;
                    break;
                }
            }
            for (int x = start_x; x < MT9V03X_W - 1; x++) {
                if (mt9v03x_image[row][x] == 0 && mt9v03x_image[row][x + 1] == 0) {
                    r = x - 1;
                    rf = 1;
                    break;
                }
            }
            if (lf && rf && r > l + 10) {
                both_center_sum += (l + r) / 2;
                both_weight_sum += 1;
                both_width_sum += (r - l);
                both_cnt++;
            } else if (lf && !rf) {
                int rr = l + s_last_width;
                if (rr > MT9V03X_W - 1) rr = MT9V03X_W - 1;
                fallback_sum += (l + rr) / 2;
                fallback_cnt++;
            } else if (!lf && rf) {
                int ll = r - s_last_width;
                if (ll < 0) ll = 0;
                fallback_sum += (ll + r) / 2;
                fallback_cnt++;
            } else if (!lf && !rf) {
                fallback_sum += start_x;
                fallback_cnt++;
            }
        }

        int total_rows = both_cnt + fallback_cnt;
        int cur_width = both_cnt > 0 ? (both_width_sum / both_cnt) : 0;
        int center_x = both_weight_sum > 0 ? (both_center_sum / both_weight_sum) : (fallback_cnt > 0 ? (fallback_sum / fallback_cnt) : start_x);
        uint8_t image_ok = (otsu_contrast >= 30 && otsu_T >= 95 && max_white_len >= 12 && total_rows > 0);
        if (cur_width > 15) {
            s_last_width = cur_width;
        }

        // step 4 直线闭环：有效帧 PD 纠偏直行；无效帧立即停车
        if (image_ok) {
            image_error = center_x - (MT9V03X_W / 2);
            image_valid = 1;

            filt_error += ERROR_LPF_ALPHA * (image_error - filt_error);
            float turn = kp * filt_error + kd * (filt_error - prev_error);
            prev_error = filt_error;

            float servo_turn = turn * SERVO_DIR;
            if (servo_turn > MAX_SERVO_TURN) servo_turn = MAX_SERVO_TURN;
            if (servo_turn < -MAX_SERVO_TURN) servo_turn = -MAX_SERVO_TURN;
            pwm_set_duty (servo, servo_center + servo_turn);

            float abs_filt = filt_error > 0 ? filt_error : -filt_error;
            int len_speed = straight_speed;
            if (max_white_len <= 50) {
                len_speed = curve_speed;
            } else if (max_white_len < 85) {
                len_speed = curve_speed + (straight_speed - curve_speed) * (max_white_len - 50) / 35;
            }
            int err_speed = straight_speed;
            if (abs_filt >= 25.0f) {
                err_speed = curve_speed;
            } else if (abs_filt > 10.0f) {
                err_speed = straight_speed - (int)((straight_speed - curve_speed) * (abs_filt - 10.0f) / 15.0f);
            }
            int target_speed = len_speed < err_speed ? len_speed : err_speed;

            float motor_turn = turn * MOTOR_SCALE;
            if (motor_turn > MAX_MOTOR_TURN) motor_turn = MAX_MOTOR_TURN;
            if (motor_turn < -MAX_MOTOR_TURN) motor_turn = -MAX_MOTOR_TURN;
            int left_pwm = target_speed + motor_turn;
            int right_pwm = target_speed - motor_turn;
            if (left_pwm > max_speed) left_pwm = max_speed;
            if (right_pwm > max_speed) right_pwm = max_speed;
            if (left_pwm < 0) left_pwm = 0;
            if (right_pwm < 0) right_pwm = 0;
            // 前进通道按本机接线取 IN2；若实测倒走，交换两对 PWM 即可
            pwm_set_duty (motor1_in1, 0);
            pwm_set_duty (motor1_in2, left_pwm);
            pwm_set_duty (motor2_in1, 0);
            pwm_set_duty (motor2_in2, right_pwm);
        } else {
            image_valid = 0;
            filt_error = 0;
            prev_error = 0;
            drive_stop();
        }

        // step 5 状态显示：1111 居中 / 333 纠正中 / LOST 停车
        // 第 2~4 行遥测：T 二值化阈值 / L 种子列白长 / W 当帧道宽(0=双边未齐) / ok 找线成功 / e 误差
        if (!image_valid) {
            tft180_show_string(0, 0, "LOST         ");
        } else {
            float abs_err = image_error > 0 ? image_error : -image_error;
            if (abs_err < CENTER_OK_RATIO * s_last_width)
                tft180_show_string(0, 0, "1111111111111");
            else
                tft180_show_string(0, 0, "33333333333  ");
        }
        tft180_show_string(0, 16, "T=");
        tft180_show_uint(16, 16, otsu_T, 3);
        tft180_show_string(48, 16, "L=");
        tft180_show_uint(64, 16, max_white_len, 3);
        tft180_show_string(0, 32, "W=");
        tft180_show_uint(16, 32, cur_width, 3);
        tft180_show_string(48, 32, "ok=");
        tft180_show_uint(72, 32, image_ok, 1);
        tft180_show_string(0, 48, "e=");
        tft180_show_int(16, 48, (int)image_error, 4);
    }
}