#ifndef IMU_H       // �����û�а��������ͷ�ļ�
#define IMU_H       // ��һ�����Ѿ��������ı��
#include "zf_common_headfile.h"
extern int16 imu660ra_acc_x,imu660ra_acc_y,imu660ra_acc_z ;
extern int16 imu660ra_gyro_x,imu660ra_gyro_y,imu660ra_gyro_z;
extern float roll,pitch,yaw;
void imu_init(void);
void imu_task(void);


#endif              // ��������ж�