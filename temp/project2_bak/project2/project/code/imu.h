#ifndef IMU_H       // 如果还没有包含过这个头文件
#define IMU_H       // 做一个“已经包含”的标记
#include "zf_common_headfile.h"
extern int16 imu660ra_acc_x,imu660ra_acc_y,imu660ra_acc_z ;
extern int16 imu660ra_gyro_x,imu660ra_gyro_y,imu660ra_gyro_z;
extern float roll,pitch,yaw;
void imu_init(void);
void imu_task(void);


#endif              // 结束这次判断