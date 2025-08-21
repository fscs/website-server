use actix_utils::future::{ready, Ready};
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{HttpMessage, HttpResponse};
use async_std::sync::Arc;
use futures_util::future::LocalBoxFuture;

use crate::domain::Capability;

use super::User;

macro_rules! capability_middleware {
    ($transform:ident, $middleware:ident, $cap:expr) => {
        pub struct $transform;

        impl<S, B> Transform<S, ServiceRequest> for $transform
        where
            S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>
                + 'static,
            S::Future: 'static,
            B: 'static,
        {
            type Response = ServiceResponse<EitherBody<B>>;
            type Error = actix_web::Error;
            type Transform = $middleware<S>;
            type InitError = ();
            type Future = Ready<Result<Self::Transform, Self::InitError>>;

            fn new_transform(&self, service: S) -> Self::Future {
                ready(Ok($middleware {
                    service: Arc::new(service),
                }))
            }
        }

        pub struct $middleware<S> {
            service: Arc<S>,
        }

        impl<S, B> Service<ServiceRequest> for $middleware<S>
        where
            S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>
                + 'static,
            S::Future: 'static,
            B: 'static,
        {
            type Response = ServiceResponse<EitherBody<B>>;
            type Error = actix_web::Error;
            type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

            forward_ready!(service);

            fn call(&self, req: ServiceRequest) -> Self::Future {
                let service = self.service.clone();
                Box::pin(async move {
                    if req
                        .extensions()
                        .get::<User>()
                        .map(ToOwned::to_owned)
                        .unwrap_or_default()
                        .has_capability($cap)
                    {
                        return Ok(service.call(req).await?.map_into_left_body());
                    }

                    return Ok(req.into_response(
                        HttpResponse::Unauthorized().finish().map_into_right_body(),
                    ));
                })
            }
        }
    };
}

capability_middleware!(
    RequireManageSitzungen,
    RequireManageSitzungenMiddleware,
    Capability::ManageSitzungen
);

capability_middleware!(
    RequireManageAnträge,
    RequireManageAnträgeMiddleware,
    Capability::ManageAnträge
);

capability_middleware!(
    RequireManagePersons,
    RequireManagePersonsMiddleware,
    Capability::ManagePersons
);

capability_middleware!(
    RequireCreateAntrag,
    RequireCreateAntragMiddleware,
    Capability::CreateAntrag
);

capability_middleware!(
    RequireViewHidden,
    RequireViewHiddenMiddleware,
    Capability::ViewHidden
);

capability_middleware!(
    RequireViewProtected,
    RequireViewProtectedMiddleware,
    Capability::ViewProtected
);
