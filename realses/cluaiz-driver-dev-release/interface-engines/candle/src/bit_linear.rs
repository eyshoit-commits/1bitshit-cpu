use candle_core::{Tensor, Result};
use candle_nn::Module;

/// 🧱 Sovereign Bit-Linear Layer (Integrated in Candle Engine)
/// Optimized for 1.58-bit (Ternary) Weights: {-1, 0, 1}
pub struct BitLinear {
    pub weights: Tensor, 
    pub scale: f32,      
}

impl BitLinear {
    pub fn new(weights: Tensor, scale: f32) -> Self {
        Self { weights, scale }
    }

    /// 🧬 BitNet 1.58b Activation Quantization
    fn quantize_activations(&self, x: &Tensor) -> Result<Tensor> {
        let abs_max = x.abs()?.max_all()?;
        let scale = 127.0 / (abs_max.to_scalar::<f32>()? + 1e-5);
        x.affine(scale as f64, 0.0)?.clamp(-128.0, 127.0)
    }

    /// 🚀 Optimized Ternary Forward Pass
    pub fn forward_native(&self, x: &Tensor) -> Result<Tensor> {
        let x_quant = self.quantize_activations(x)?;
        let out = x_quant.matmul(&self.weights.transpose(0, 1)?)?;
        out.affine(1.0 / self.scale as f64, 0.0)
    }
}

impl Module for BitLinear {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        self.forward_native(x)
    }
}
