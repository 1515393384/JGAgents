################################################################################
# MRS Version: 2.5.0
# Automatically-generated file. Do not edit!
################################################################################

# Add inputs and outputs from these tool invocations to the build variables 
C_SRCS += \
d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Core/core_riscv.c 

C_DEPS += \
./sdk/Core/core_riscv.d 

OBJS += \
./sdk/Core/core_riscv.o 

DIR_OBJS += \
./sdk/Core/*.o \

DIR_DEPS += \
./sdk/Core/*.d \

DIR_EXPANDS += \
./sdk/Core/*.234r.expand \


# Each subdirectory must supply rules for building sources it contributes
sdk/Core/core_riscv.o: d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Core/core_riscv.c
	@	riscv-none-embed-gcc -march=rv32imafc -mabi=ilp32f -msmall-data-limit=8 -mno-save-restore -fmax-errors=20 -O0 -fmessage-length=0 -fsigned-char -ffunction-sections -fdata-sections -fno-common -pedantic -Wunused -Wuninitialized -Wall -g -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/Libraries/doc" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/zf_components" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Core" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Ld" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Peripheral" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/sdk/Startup" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/project/user/inc" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/zf_common" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/zf_device" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/project/code" -I"d:/LeStoreDownload/CHV307_Library-master/CHV307_Library-master/2026930718何金桐/libraries/zf_driver" -std=gnu11 -MMD -MP -MF"$(@:%.o=%.d)" -MT"$(@)" -c -o "$@" "$<"

