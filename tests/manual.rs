use approx::*;
use arraymap::ArrayMap;
use arrsac::{Arrsac, Config};
use cv::nalgebra::{Isometry3, Point2, Point3, Translation, UnitQuaternion, Vector3};
use cv::KeypointWorldMatch;
use p3p::nordberg::NordbergEstimator;
use pnp::pnp;
use rand::{rngs::SmallRng, Rng, SeedableRng};

const EPSILON_APPROX: f32 = 1e-2;
const NOISE_LEVEL: f32 = 1e-4;
const NOISY_ITERATIONS: usize = 256;

#[test]
fn noiseless() {
    manual_test_mutator(|p| p);
}

#[test]
fn noisy() {
    let mut rng = SmallRng::from_seed([0; 16]);
    for _ in 0..NOISY_ITERATIONS {
        manual_test_mutator(|p| {
            p.coords
                .map(|n| n + NOISE_LEVEL * (rng.gen::<f32>() - 0.5))
                .into()
        });
    }
}

fn manual_test_mutator(mut mutator: impl FnMut(Point2<f32>) -> Point2<f32>) {
    // Define some points in camera coordinates (with z > 0).
    let camera_depth_points = [
        [-0.228_125, -0.061_458_334, 1.0],
        [0.418_75, -0.581_25, 2.0],
        [1.128_125, 0.878_125, 3.0],
        [-0.528_125, 0.178_125, 2.5],
        [0.514_125, -0.748_125, 1.2],
        [0.114_125, -0.247_125, 3.7],
    ]
    .map(|&p| Point3::from(p));

    // Define the camera pose.
    let rot = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
    let trans = Translation::from(Vector3::new(0.1, 0.2, 0.3));
    let pose = Isometry3::from_parts(trans, rot);

    // Compute world coordinates.
    let world_points = camera_depth_points.map(|p| pose.inverse() * p);

    // Compute normalized image coordinates.
    let mut normalized_image_coordinates = camera_depth_points.map(|p| (p / p.z).xy());

    // Mutate image coordinates
    for coord in &mut normalized_image_coordinates {
        *coord = mutator(*coord);
    }

    let samples: Vec<KeypointWorldMatch> = world_points
        .iter()
        .zip(&normalized_image_coordinates)
        .map(|(&world, &image)| KeypointWorldMatch(image.into(), world.into()))
        .collect();

    let pose = pnp(
        NordbergEstimator,
        Arrsac::new(Config::new(0.01), SmallRng::from_seed([0; 16])),
        samples.iter().copied(),
    )
    .unwrap();

    // Compare the pose to ground truth.
    assert_relative_eq!(rot, pose.rotation, epsilon = EPSILON_APPROX);
    assert_relative_eq!(trans, pose.translation, epsilon = EPSILON_APPROX);
}
