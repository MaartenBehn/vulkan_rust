sh ./scripts/compile_shaders_selected.sh $1
sh ./scripts/release.sh $1 
./target/release/$1
