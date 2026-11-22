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

#include "zf_common_headfile.h"
#define servo TIM2_PWM_MAP1_CH1_A15
#define motor1_in1 TIM4_PWM_MAP1_CH1_D12
#define motor1_in2 TIM4_PWM_MAP1_CH2_D13
#define motor2_in1 TIM1_PWM_MAP3_CH2_E11
#define motor2_in2 TIM1_PWM_MAP3_CH1_E9

#define servo_center 750
#define dir_sign 1
#define kp 15.0f
#define kd 5.0f
#define base_speed 2000
#define max_turn 150
#define max_speed 3500
#define MOTOR_SCALE 3.0f
#define SERVO_SCALE 1.0F
#define MAX_MOTOR_TURN 2000
#define MAX_SERVO_TURN 150
// 供中断使用的全局变量
float g_image_error = 0;    // 图像偏差
uint8_t g_image_valid = 0;  // 图像是否有效（1有效，0无效）

// 大津法
uint8_t get_otsu_threshold (uint8_t image[][MT9V03X_W], int width, int height) {
    int histogram[256] = {0};
    int total_pixels = width * height;
    float sum = 0;

    for (int i = 0; i < height; i++) {
        for (int j = 0; j < width; j++) {
            histogram[image[i][j]]++;
            sum += image[i][j];
        }
    }

    float sumB = 0;
    int wB = 0;
    int wF = 0;
    float varMax = 0;
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
        float varBetween = (float)wB * (float)wF * (mB - mF) * (mB - mF);
        if (varBetween > varMax) {
            varMax = varBetween;
            threshold = t;
        }
    }
    return threshold;
}

int main (void) {
    clock_init (SYSTEM_CLOCK_120M);  // 初始化芯片时钟 工作频率为 120MHz
    debug_init();                    // 务必保留，本函数用于初始化MPU 时钟 调试串口
    // 此处编写用户代码 例如外设初始化代码等
    mt9v03x_init();
    tft180_init();
    pwm_init (servo, 50, servo_center);  // servoz中值
    pwm_init (motor1_in1, 17000, 0);
    pwm_init (motor1_in2, 17000, 0);
    pwm_init (motor2_in1, 17000, 0);
    pwm_init (motor2_in2, 17000, 0);
    uint8_t otsu_T = 0;

    // 此处编写用户代码 例如外设初始化代码等
    while (1) {
        // 此处编写需要循环执行的代码

        // 当摄像头拍完一张图
        if (mt9v03x_finish_flag) {
            mt9v03x_finish_flag = 0;  // 清空标志

            // 1. 让单片机用大津法算出一个阈值（分界线）
            otsu_T = get_otsu_threshold (mt9v03x_image, MT9V03X_W, MT9V03X_H);

            // 2. 把图片变成纯黑纯白
            for (int i = 0; i < MT9V03X_H; i++) {
                for (int j = 0; j < MT9V03X_W; j++) {
                    if (mt9v03x_image[i][j] > otsu_T) {
                        mt9v03x_image[i][j] = 255;  // 比阈值亮，变成纯白
                    } else {
                        mt9v03x_image[i][j] = 0;    // 比阈值暗，变成纯黑
                    }
                }
            }
            int max_white_len = 0;        // 记录最长的白线长度
            int start_x = MT9V03X_W / 2;  // 默认出发点在屏幕正中间

            // 从第0列扫到最后一列
            for (int x = 0; x < MT9V03X_W; x++) {
                int current_len = 0;
                // 从最底下往上扫
                for (int y = MT9V03X_H - 1; y >= 0; y--) {
                    if (mt9v03x_image[y][x] == 255) {  // 是白点
                        current_len++;                 // 白线长度+1
                    } else {
                        break;                         // 碰到黑点，这列断开，退出
                    }
                }
                // 如果这一列的白线更长，更新出发点
                if (current_len > max_white_len) {
                    max_white_len = current_len;
                    start_x = x;
                }
            }

            // 看从下往上数第 20 行的位置
            int y_look = MT9V03X_H - 20;

            // 从出发点 start_x 向左右找边界 ---
            int left_edge = start_x;
            int right_edge = start_x;

            // 向左找黑点
            for (int x = start_x; x >= 0; x--) {
                if (mt9v03x_image[y_look][x] == 0) {
                    left_edge = x + 1;  // 碰到黑点，记录边界
                    break;
                }
            }
            // 向右找黑点
            for (int x = start_x; x < MT9V03X_W; x++) {
                if (mt9v03x_image[y_look][x] == 0) {
                    right_edge = x - 1;  // 碰到黑点，记录边界
                    break;
                }
            }

            // 计算中心点与偏差 ---
            int center_x = (left_edge + right_edge) / 2;  // 赛道中心点
            int e = center_x - (MT9V03X_W / 2);           // 偏差（红点位置 - 屏幕中心）
            g_image_error = e;
            g_image_valid = 1;
            // 把黑白图显示在屏幕上
            tft180_displayimage03x (mt9v03x_image[0], MT9V03X_W, MT9V03X_H);
            // 画边界和中心点
            tft180_draw_point (center_x, y_look, RGB565_RED);      // 画红点（中心）
            tft180_draw_point (left_edge, y_look, RGB565_GREEN);   // 画绿点（左边界）
            tft180_draw_point (right_edge, y_look, RGB565_GREEN);  // 画绿点（右边界）
        }

        // 此处编写需要循环执行的代码
    }
}