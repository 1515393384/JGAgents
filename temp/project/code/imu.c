#include "imu.h"
#include <stdio.h>
#include "zf_common_headfile.h"
#include "math.h"
#include "flash.h"

// ȫ�ֱ�������
float roll = 0, pitch = 0, yaw = 0;
uint64_t last_time = 0;
extern float my_float;
float gro_x_offest = 0;
float gro_y_offest = 0;
float gro_z_offest = 0;

// ����ʵ��
void imu_init (void) {
    // ��ʼ������
    imu660ra_init();
    float sum_x = 0, sum_y = 0, sum_z = 0;
    for (int i = 0; i < 200; i++) {
        imu660ra_get_gyro();
        sum_x += imu660ra_gyro_transition (imu660ra_gyro_x);
        sum_y += imu660ra_gyro_transition (imu660ra_gyro_y);
        sum_z += imu660ra_gyro_transition (imu660ra_gyro_z);
        system_delay_ms (10);
    }
    gro_x_offest = sum_x / 200.0f;
    gro_y_offest = sum_y / 200.0f;
    gro_z_offest = sum_z / 200.0f;
    last_time = SysTick->CNT;
}

void imu_task (void) {
    imu660ra_get_acc();
    imu660ra_get_gyro();
    float dt=0.01f;
   



    float acc_x = imu660ra_acc_transition (imu660ra_acc_x);
    float acc_y = -imu660ra_acc_transition (imu660ra_acc_y);
    float acc_z = -imu660ra_acc_transition (imu660ra_acc_z);
    float acc_roll = atan2 (acc_y, acc_z) * 57.29578f;
    float acc_pitch = atan2 (acc_x, sqrtf (acc_y * acc_y + acc_z * acc_z)) * 57.29578f;
    float gyro_roll_rate = imu660ra_gyro_transition (imu660ra_gyro_x) - gro_x_offest;
    float gyro_pitch_rate = -imu660ra_gyro_transition (imu660ra_gyro_y) + gro_y_offest;
    float gyro_yaw_rate = -imu660ra_gyro_transition (imu660ra_gyro_z) + gro_z_offest;
    roll = my_float * (roll + gyro_roll_rate * dt) + (1.0f - my_float) * acc_roll;
    pitch = my_float * (pitch + gyro_pitch_rate * dt) + (1.0f - my_float) * acc_pitch;

    yaw += gyro_yaw_rate * dt;

    while (roll  >  180.0f) roll  -= 360.0f;
    while (roll  < -180.0f) roll  += 360.0f;
    while (pitch >  180.0f) pitch -= 360.0f;
    while (pitch < -180.0f) pitch += 360.0f;
    while (yaw   >  180.0f) yaw   -= 360.0f;
    while (yaw   < -180.0f) yaw   += 360.0f;
}