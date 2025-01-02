U_FAT32_DIR="../fat32-fuse"
U_FAT32="${U_FAT32_DIR}/fat32.img"

sudo chmod 777 ${U_FAT32}
sudo umount ${U_FAT32}
#sudo umount ${U_FAT32}
sudo mkfs.vfat -F 32 ${U_FAT32}

if test -e ${U_FAT32_DIR}/fs
then 
    sudo rm -r ${U_FAT32_DIR}/fs
    mkdir ${U_FAT32_DIR}/fs
else
    mkdir ${U_FAT32_DIR}/fs
fi


sudo mount ${U_FAT32} ${U_FAT32_DIR}/fs 

#sudo rm ${U_FAT32_DIR}/fs/*

#for programname in $(ls ../user/src/bin)
#do
#    if [ $programname != "initproc.rs" ] #&& [ $programname != "user_shell.rs" ]
#    then 
#        sudo cp ../user/target/riscv64gc-unknown-none-elf/release/${programname%.rs} ../fat32-fuse/fs/${programname%.rs}
#    fi
#done
sudo cp ../user_C_program/user/build/riscv64 ../fat32-fuse/fs/ -r

sudo umount ${U_FAT32_DIR}/fs
