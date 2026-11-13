use super::{Image, Rect, Renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorError {
    CapacityReached,
    InvalidLayerIndex,
}

#[derive(Clone, Copy)]
struct Layer<'pixels> {
    image: Image<'pixels>,
    bounds: Rect,
    visible: bool,
}

pub struct Compositor<'pixels, const MAX_LAYERS: usize = 16> {
    layers: [Option<Layer<'pixels>>; MAX_LAYERS],

    len: usize,
}

impl<'pixels, const MAX_LAYERS: usize> Compositor<'pixels, MAX_LAYERS> {
    pub fn new() -> Self {
        Self {
            layers: [None; MAX_LAYERS],
            len: 0,
        }
    }

    pub const fn layer_count(&self) -> usize {
        self.len
    }

    pub fn add_layer(
        &mut self,
        image: Image<'pixels>,
        bounds: Rect,
    ) -> Result<usize, CompositorError> {
        if self.len >= MAX_LAYERS {
            return Err(CompositorError::CapacityReached);
        }

        let index = self.len;

        self.layers[index] = Some(Layer {
            image,
            bounds,
            visible: true,
        });

        self.len += 1;

        Ok(index)
    }

    pub fn remove_layer(&mut self, index: usize) -> Result<(), CompositorError> {
        if index >= self.len {
            return Err(CompositorError::InvalidLayerIndex);
        }

        for current in index..self.len - 1 {
            self.layers[current] = self.layers[current + 1];
        }

        self.len -= 1;
        self.layers[self.len] = None;

        Ok(())
    }

    pub fn set_visible(&mut self, index: usize, visible: bool) -> Result<(), CompositorError> {
        let Some(layer) = self.layers.get_mut(index) else {
            return Err(CompositorError::InvalidLayerIndex);
        };

        let Some(layer) = layer.as_mut() else {
            return Err(CompositorError::InvalidLayerIndex);
        };

        layer.visible = visible;

        Ok(())
    }

    pub fn set_bounds(&mut self, index: usize, bounds: Rect) -> Result<(), CompositorError> {
        let Some(layer) = self.layers.get_mut(index) else {
            return Err(CompositorError::InvalidLayerIndex);
        };

        let Some(layer) = layer.as_mut() else {
            return Err(CompositorError::InvalidLayerIndex);
        };

        layer.bounds = bounds;

        Ok(())
    }

    pub fn move_to_font(&mut self, index: usize) -> Result<(), CompositorError> {
        if index >= self.len {
            return Err(CompositorError::InvalidLayerIndex);
        };

        let layer = self.layers[index];

        for current in index..self.len - 1 {
            self.layers[current] = self.layers[current + 1];
        }

        self.layers[self.len - 1] = layer;

        Ok(())
    }

    pub fn clear(&mut self) {
        for layer in &mut self.layers {
            *layer = None;
        }

        self.len = 0;
    }

    pub fn render(&self, renderer: &mut Renderer) {
        for index in 0..self.len {
            let Some(layer) = self.layers[index] else {
                continue;
            };

            if !layer.visible {
                continue;
            }

            layer.image.draw_scaled(renderer, layer.bounds);
        }
    }
}
