use octa_force::glam::Mat3;

/// Origanl from https://docs.rs/dot_vox/latest/dot_vox/struct.Rotation.html
///
/// A **[`Signed Permutation Matrix`]** [^note] encoded in a byte.
///
/// # Encoding
/// The encoding follows the MagicaVoxel [ROTATION] type.
///
/// for example :
/// ```
/// let R = [
///   [0,  1,  0],
///   [0,  0, -1],
///   [-1, 0,  0],
/// ];
/// let _r: u8 = (1 << 0) | (2 << 2) | (0 << 4) | (1 << 5) | (1 << 6);
/// ```
///
/// | bit | value |                  descripton                             |
/// |-----|-------|---------------------------------------------------------|
/// | 0-1 |   1   | Index of the non-zero entry in the first row            |
/// | 2-3 |   2   | Index of the non-zero entry in the second row           |
/// |  4  |   0   | The sign in the first row (0 - positive; 1 - negative)|
/// |  5  |   1   | The sign in the second row  (0 - positive; 1 - negative)|
/// |  6  |   1   | The sign in the third row  (0 - positive; 1 - negative) |
///
/// [`Signed Permutation Matrix`]: https://en.wikipedia.org/wiki/Generalized_permutation_matrix#Signed_permutation_group
/// [ROTATION]: https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox-extension.txt#L24
/// [^note]: A [`Signed Permutation Matrix`] is a square binary matrix that has exactly one entry of ±1 in each row and each column and 0s elsewhere.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rot(u8);

pub type Quat = [f32; 4];
pub type Vec3 = [f32; 3];

impl Rot {
    pub const IDENTITY: Self = Rot(0b0000100);

    /// Decompose the Signed Permutation Matrix into a rotation component, represented by a Quaternion,
    /// and a flip component, represented by a Vec3 which is either Vec3::ONE or -Vec3::ONE.
    pub fn to_quat_scale(&self) -> (Quat, Vec3) {
        let index_nz1 = self.0 & 0b11;
        let index_nz2 = (self.0 >> 2) & 0b11;
        let flip = (self.0 >> 4) as usize;

        let si = [1.0, 1.0, 1.0]; // scale_identity
        let sf = [-1.0, -1.0, -1.0]; // scale_flip
        const SQRT_2_2: f32 = std::f32::consts::SQRT_2 / 2.0;
        match (index_nz1, index_nz2) {
            (0, 1) => {
                let quats = [
                    [0.0, 0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                ];
                let mapping = [0, 3, 2, 1, 1, 2, 3, 0];
                let scales = [si, sf, sf, si, sf, si, si, sf];
                (quats[mapping[flip]], scales[flip])
            }
            (0, 2) => {
                let quats = [
                    [0.0, SQRT_2_2, SQRT_2_2, 0.0],
                    [SQRT_2_2, 0.0, 0.0, SQRT_2_2],
                    [SQRT_2_2, 0.0, 0.0, -SQRT_2_2],
                    [0.0, SQRT_2_2, -SQRT_2_2, 0.0],
                ];
                let mapping = [3, 0, 1, 2, 2, 1, 0, 3];
                let scales = [sf, si, si, sf, si, sf, sf, si];
                (quats[mapping[flip]], scales[flip])
            }
            (1, 2) => {
                let quats = [
                    [0.5, 0.5, 0.5, -0.5],
                    [0.5, -0.5, 0.5, 0.5],
                    [0.5, -0.5, -0.5, -0.5],
                    [0.5, 0.5, -0.5, 0.5],
                ];
                let mapping = [0, 3, 2, 1, 1, 2, 3, 0];
                let scales = [si, sf, sf, si, sf, si, si, sf];
                (quats[mapping[flip]], scales[flip])
            }
            (1, 0) => {
                let quats = [
                    [0.0, 0.0, SQRT_2_2, SQRT_2_2],
                    [0.0, 0.0, SQRT_2_2, -SQRT_2_2],
                    [SQRT_2_2, SQRT_2_2, 0.0, 0.0],
                    [SQRT_2_2, -SQRT_2_2, 0.0, 0.0],
                ];
                let mapping = [3, 0, 1, 2, 2, 1, 0, 3];
                let scales = [sf, si, si, sf, si, sf, sf, si];

                (quats[mapping[flip]], scales[flip])
            }
            (2, 0) => {
                let quats = [
                    [0.5, 0.5, 0.5, 0.5],
                    [0.5, -0.5, -0.5, 0.5],
                    [0.5, 0.5, -0.5, -0.5],
                    [0.5, -0.5, 0.5, -0.5],
                ];
                let mapping = [0, 3, 2, 1, 1, 2, 3, 0];
                let scales = [si, sf, sf, si, sf, si, si, sf];
                (quats[mapping[flip]], scales[flip])
            }
            (2, 1) => {
                let quats = [
                    [0.0, SQRT_2_2, 0.0, -SQRT_2_2],
                    [SQRT_2_2, 0.0, SQRT_2_2, 0.0],
                    [0.0, SQRT_2_2, 0.0, SQRT_2_2],
                    [SQRT_2_2, 0.0, -SQRT_2_2, 0.0],
                ];
                let mapping = [3, 0, 1, 2, 2, 1, 0, 3];
                let scales = [sf, si, si, sf, si, sf, sf, si];
                (quats[mapping[flip]], scales[flip])
            }
            _ => unreachable!(),
        }
    }
}

impl From<u8> for Rot {
    fn from(byte: u8) -> Self {
        let index_nz1 = byte & 0b11;
        let index_nz2 = (byte >> 2) & 0b11;
        assert!(
            (index_nz1 != index_nz2) && (index_nz1 != 0b11 && index_nz2 != 0b11),
            "Invalid Rotation"
        );
        Rot(byte)
    }
}

impl Into<Mat3> for Rot {
    fn into(self) -> Mat3 {
        let mut cols: [[f32; 3]; 3] = [[0.0; 3]; 3];

        let index_nz1 = self.0 & 0b11;
        let index_nz2 = (self.0 >> 2) & 0b11;
        let index_nz3 = 3 - index_nz1 - index_nz2;

        let row_1_sign: f32 = if self.0 & (1 << 4) == 0 { 1.0 } else { -1.0 };
        let row_2_sign: f32 = if self.0 & (1 << 5) == 0 { 1.0 } else { -1.0 };
        let row_3_sign: f32 = if self.0 & (1 << 6) == 0 { 1.0 } else { -1.0 };

        cols[index_nz1 as usize][0] = row_1_sign;
        cols[index_nz2 as usize][1] = row_2_sign;
        cols[index_nz3 as usize][2] = row_3_sign;

        Mat3::from_cols_array_2d(&cols)
    }
}

impl Into<u8> for Rot {
    fn into(self) -> u8 {
        self.0
    }
}

impl std::fmt::Debug for Rot {
    /// Print the Rotation in a format that looks like `Rotation(-y, -z, x)`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        let index_nz1 = self.0 & 0b11;
        let index_nz2 = (self.0 >> 2) & 0b11;
        let index_nz3 = 3 - index_nz1 - index_nz2;

        let xyz: &[char; 3] = &['x', 'y', 'z'];

        f.write_str("Rotation(")?;

        if self.0 & (1 << 4) != 0 {
            f.write_char('-')?;
        };
        f.write_char(xyz[index_nz1 as usize])?;
        f.write_char(' ')?;

        if self.0 & (1 << 5) != 0 {
            f.write_char('-')?;
        };
        f.write_char(xyz[index_nz2 as usize])?;
        f.write_char(' ')?;

        if self.0 & (1 << 6) != 0 {
            f.write_char('-')?;
        };
        f.write_char(xyz[index_nz3 as usize])?;
        f.write_char(')')?;
        Ok(())
    }
}

impl std::ops::Mul<Rot> for Rot {
    type Output = Rot;

    /// Integer-only multiplication of two Rotation.
    fn mul(self, rhs: Rot) -> Rot {
        let mut rhs_rows = [rhs.0 & 0b11, (rhs.0 >> 2) & 0b11, 0];
        rhs_rows[2] = 3 - rhs_rows[0] - rhs_rows[1];

        let mut lhs_rows = [self.0 & 0b11, (self.0 >> 2) & 0b11, 0];
        lhs_rows[2] = 3 - lhs_rows[0] - lhs_rows[1];
        let lhs_signs = self.0 >> 4;

        let result_row_0 = rhs_rows[lhs_rows[0] as usize];
        let result_row_1 = rhs_rows[lhs_rows[1] as usize];
        let rhs_signs = rhs.0 >> 4;

        let rhs_signs_0 = (rhs_signs >> lhs_rows[0]) & 1;
        let rhs_signs_1 = (rhs_signs >> lhs_rows[1]) & 1;
        let rhs_signs_2 = (rhs_signs >> lhs_rows[2]) & 1;
        let rhs_signs_permutated: u8 = rhs_signs_0 | (rhs_signs_1 << 1) | (rhs_signs_2 << 2);
        let signs = lhs_signs ^ rhs_signs_permutated;
        Rot(result_row_0 | (result_row_1 << 2) | (signs << 4))
    }
}

impl Default for Rot {
    fn default() -> Self {
        Self::from(4)
    }
}
