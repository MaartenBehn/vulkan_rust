# Vulkan

A side repo where I try a bunch of different rendering technics and Ideas with vulkan in rust.


## Examples
|  |  |
-------------------------|-------------------------
**Dynamik Voxel loader and raycasting** | 
`sh scripts/run_voxel_renderer.sh` | 
![Pic](assets/screenshots/voxel_renderer.png)  |  
**Mirror scene with raytracing** | **Global ilumination scene raytracing**
`sh scripts/run.sh rt_mirror` | `sh scripts/run.sh rt_global_ilumination`
![Pic](assets/screenshots/mirror.png)  |  ![Pic](assets/screenshots/global_ilumination.png) 
**Particle Simulation**  | **Mandelbrot Compute Shader** 
`sh scripts/run.sh gpu_particles` | `sh scripts/run.sh mandelbrot`
![Pic](assets/screenshots/particles.png)  |  ![Pic](assets/screenshots/mandelbrot.png)



## Install
### Dependecies

- Rust
- cmake
- pkg-config
- glslangValidator
- vulkan
- hotspot  
*(for profiling)*
- cargo depgraph  
*(for showing dependecies)*


### Installing Dependecies on Ubuntu LTS 20.04
#### [Rust](https://www.rust-lang.org/tools/install)
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
#### cmake
```bash
sudo apt install cmake
```
#### pkg-config
```bash
sudo apt install libfontconfig-dev
```
#### glslangValidator
```bash
sudo apt-get install glslang-tools
```
#### [Vulkan](https://vulkan.lunarg.com/doc/view/latest/linux/getting_started_ubuntu.html)
```bash
wget -qO - http://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-focal.list http://packages.lunarg.com/vulkan/lunarg-vulkan-focal.list
sudo apt update
sudo apt install vulkan-sdk
```





