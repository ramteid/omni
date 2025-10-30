#!/bin/bash
set -ex

# Log to file for debugging
exec > >(tee /var/log/user-data.log)
exec 2>&1

echo "Starting user data script..."

# Configure ECS cluster
echo "Configuring ECS cluster: ${cluster_name}"
cat <<EOF >> /etc/ecs/ecs.config
ECS_CLUSTER=${cluster_name}
ECS_ENABLE_TASK_IAM_ROLE=true
ECS_ENABLE_TASK_IAM_ROLE_NETWORK_HOST=true
EOF

# Wait for the EBS volume to be attached (with timeout)
echo "Waiting for EBS volume ${device_name} to be available..."
COUNTER=0
until [ -e ${device_name} ] || [ $COUNTER -eq 60 ]; do
  sleep 1
  COUNTER=$((COUNTER + 1))
done

if [ ! -e ${device_name} ]; then
  echo "ERROR: EBS volume ${device_name} not found after 60 seconds"
  exit 1
fi

echo "EBS volume found: ${device_name}"

# Check if the volume has a filesystem
if ! blkid ${device_name}; then
  echo "Creating ext4 filesystem on ${device_name}..."
  mkfs.ext4 -F ${device_name}
else
  echo "Filesystem already exists on ${device_name}"
fi

# Create mount point
echo "Creating mount point: ${mount_point}"
mkdir -p ${mount_point}

# Mount the volume
echo "Mounting ${device_name} to ${mount_point}..."
mount ${device_name} ${mount_point}

# Add to fstab for persistence across reboots (use UUID for reliability)
UUID=$(blkid -s UUID -o value ${device_name})
if ! grep -q "$UUID" /etc/fstab; then
  echo "UUID=$UUID ${mount_point} ext4 defaults,nofail 0 2" >> /etc/fstab
  echo "Added to fstab with UUID: $UUID"
fi

# Remove lost+found directory created by ext4 filesystem
# PostgreSQL won't initialize in a non-empty directory
echo "Removing lost+found directory if it exists..."
rm -rf ${mount_point}/lost+found

# Set proper permissions for PostgreSQL (UID 999 is postgres in the container)
echo "Setting permissions on ${mount_point}..."
chown -R 999:999 ${mount_point}
chmod 700 ${mount_point}

# Start ECS agent with --no-block to avoid circular dependency with cloud-init
echo "Starting ECS agent..."
systemctl enable --now --no-block ecs

echo "User data script completed successfully!"
