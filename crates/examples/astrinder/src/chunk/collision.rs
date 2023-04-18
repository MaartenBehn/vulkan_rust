/* 
    pub fn check_collision_with_particle(&self, chunk_pos: Vec2, check_pos: Vec2) -> bool {
        self.particles.query_around((check_pos - chunk_pos).to_array(), PARTICLE_RADIUS).peekable().peek().is_some()
    }

    pub fn check_collision_with_chunk(&self, chunk_pos: Vec2, other_chunk: &Chunk, other_pos: Vec2) -> bool {
        for (pos, _) in self.particles.objects(){

            let check_pos = Vec2::from_array(pos) + chunk_pos - other_pos;
            if other_chunk.particles.query_around(check_pos.to_array(), PARTICLE_RADIUS).peekable().peek().is_some(){
                return true;
            }
        }
        return false;
    }
    */